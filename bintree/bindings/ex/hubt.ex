defmodule HUBT do
  @zero_hash :binary.copy(<<0>>, 32)

  def init() do
    :ets.new(THUBT, [
      :ordered_set, :named_table, :public,
      {:write_concurrency, true}, {:read_concurrency, true}
    ])
  end

  def root() do
    case :ets.first(THUBT) do
      :"$end_of_table" -> @zero_hash
      key -> [{_, hash}] = :ets.lookup(THUBT, key); hash
    end
  end

  # --- BATCH UPDATE (Canonical / Collapsed) ---
  def batch_update(ops) do
    prepared_ops = ops
    |> Enum.map(fn
      {:insert, k, v} -> {:insert, :crypto.hash(:sha256, k), :crypto.hash(:sha256, k <> v)}
      {:delete, k, _} -> {:delete, :crypto.hash(:sha256, k)}
    end)
    |> Enum.sort_by(fn {_, p, _} -> p; {_, p} -> p end)

    # 1. Delete old leaves
    Enum.each(prepared_ops, fn
      {:delete, p} -> :ets.delete(THUBT, {:n, p, 256})
      _ -> :ok
    end)

    # 2. Insert new leaves
    objects = Enum.flat_map(prepared_ops, fn
      {:insert, p, l} -> [{{:n, p, 256}, l}]
      _ -> []
    end)
    unless objects == [], do: :ets.insert(THUBT, objects)

    # 3. Ensure structure & 4. Prune bottom-up
    Enum.each(prepared_ops, fn
      {:insert, p, l} -> ensure_split_points(p, l)
      _ -> :ok
    end)

    collect_dirty_ancestors(prepared_ops)
    |> rehash_and_prune_batch()
  end

  defp collect_dirty_ancestors(ops) do
    Enum.reduce(ops, MapSet.new(), fn op, acc ->
      path = elem(op, 1)
      changes_path_from_lcp(path, 255, [])
      |> Enum.reduce(acc, fn {key, _}, set -> MapSet.put(set, key) end)
    end)
  end

  defp ensure_split_points(path, leaf) do
    key = {:n, path, 256}
    check_neighbor(:ets.prev_lookup(THUBT, key), path, leaf)
    check_neighbor(:ets.next_lookup(THUBT, key), path, leaf)
  end

  defp check_neighbor({{_, n_path, 256}, [{_, n_leaf}]}, path, leaf) do
    {lcp_path, len} = lcp(path, n_path)
    :ets.insert(THUBT, {{:n, lcp_path, len}, :crypto.hash(:sha256, leaf <> n_leaf)})
  end
  defp check_neighbor(_, _, _), do: :ok

  defp rehash_and_prune_batch(dirty_nodes) do
    dirty_nodes
    |> Enum.to_list()
    |> Enum.sort_by(fn {:n, _, len} -> len end, :desc)
    |> Enum.each(fn {:n, path, len} ->
      l_hash = get_child_hash(path, len, 0)
      r_hash = get_child_hash(path, len, 1)

      # CANONICAL LOGIC: Node exists ONLY if it branches (both children exist)
      if l_hash != @zero_hash and r_hash != @zero_hash do
        :ets.insert(THUBT, {{:n, path, len}, :crypto.hash(:sha256, l_hash <> r_hash)})
      else
        :ets.delete(THUBT, {:n, path, len})
      end
    end)
  end

  # --- TRAVERSAL & HELPERS ---
  def changes_path_from_lcp(path, len, acc), do: walk_changes({:n, path, len + 1}, path, acc)

  defp walk_changes(cursor, target, acc) do
    case :ets.prev(THUBT, cursor) do
      {:n, path, len} = key ->
        if prefix_match?(target, path, len) do
          [{_, hash}] = :ets.lookup(THUBT, key)
          walk_changes(key, target, acc ++ [{key, hash}])
        else
          {lcp_path, lcp_len} = lcp(path, target)
          walk_changes(min({:n, lcp_path, lcp_len + 1}, key), target, acc)
        end
      _ -> acc
    end
  end

  def closest_or_next_lookup(path, len) do
    key = {:n, path, len}
    case :ets.lookup(THUBT, key) do
      [{^key, hash}] -> {key, hash}
      [] -> 
        case :ets.next_lookup(THUBT, key) do
          {k, [{_, h}]} -> {k, h}
          _ -> {nil, nil}
        end
    end
  end

  defp get_child_hash(p_path, p_len, dir) do
    <<prefix::bitstring-size(p_len), _::bitstring>> = p_path
    target = <<prefix::bitstring, dir::1, 0::size(255 - p_len)>>
    t_len = p_len + 1
    
    case closest_or_next_lookup(target, t_len) do
      {{:n, fp, _}, hash} ->
        <<t_pre::size(t_len), _::bitstring>> = target
        <<f_pre::size(t_len), _::bitstring>> = fp
        if t_pre == f_pre, do: hash, else: @zero_hash
      _ -> @zero_hash
    end
  end

  def lcp(p1, p2) do
    len = do_lcp_len(p1, p2, 0)
    <<prefix::bitstring-size(len), _::bitstring>> = p1
    {pad_to_256(prefix), len}
  end
  defp do_lcp_len(<<b::1, r1::bitstring>>, <<b::1, r2::bitstring>>, acc), do: do_lcp_len(r1, r2, acc + 1)
  defp do_lcp_len(_, _, acc), do: acc

  defp prefix_match?(target, path, len) do
    len <= bit_size(target) and (<<t::size(len), _::bitstring>> = target; <<p::size(len), _::bitstring>> = path; t == p)
  end

  def pad_to_256(bs) do
    missing = 256 - bit_size(bs)
    if missing > 0, do: <<bs::bitstring, 0::size(missing)>>, else: bs
  end

  # --- PROOFS & VERIFICATION ---

  # 1. INCLUSION PROOF
  def prove(k, v) do
    path = :crypto.hash(:sha256, k)
    leaf = :crypto.hash(:sha256, k <> v)

    case :ets.lookup(THUBT, {:n, path, 256}) do
      [{_, val}] when val == leaf ->
        %{type: :inclusion, root: root(), nodes: generate_proof_nodes(path, 256)}
      _ -> %{error: :not_found}
    end
  end

  def verify(k, v, proof) do
    leaf = :crypto.hash(:sha256, k <> v)
    calculate_root(leaf, proof.nodes) == proof.root
  end

  # 2. EXCLUSION PROOF (Divergence)
  def prove_non_existence(k) do
    target = :crypto.hash(:sha256, k)
    {best_key, best_hash} = find_longest_prefix_node(target)
    
    case best_key do
      nil -> %{type: :non_existence, proof: %{root: @zero_hash, nodes: []}}
      {:n, path, len} ->
        if len == 256 and path == target do
           %{error: :key_exists}
        else
           %{
             type: :non_existence,
             proven_path: path,
             proven_hash: best_hash,
             proof: %{root: root(), nodes: generate_proof_nodes(path, len)}
           }
        end
    end
  end

  def verify_non_existence(k, proof) do
    target = :crypto.hash(:sha256, k)
    if proof.proof.root == @zero_hash do
      proof.proof.nodes == []
    else
      # Integrity: Reconstruct root from Proven Hash
      root_ok = (calculate_root(proof.proven_hash, proof.proof.nodes) == proof.proof.root)

      # Logic: Divergence & Ambiguity Check
      div_idx = do_divergence_index(proof.proven_path, target, 0)
      ambiguous = Enum.any?(proof.proof.nodes, fn node -> node.len == div_idx end)
      
      root_ok and (proof.proven_path != target) and (not ambiguous)
    end
  end

  # 3. MISMATCH PROOF (Key exists, value differs)
  def prove_mismatch(k, v_claimed) do
    path = :crypto.hash(:sha256, k)
    case :ets.lookup(THUBT, {:n, path, 256}) do
      [] -> %{error: :key_not_found}
      [{_, actual}] ->
        if actual == :crypto.hash(:sha256, k <> v_claimed), do: %{error: :value_matches}, else:
        %{
          type: :mismatch,
          actual_hash: actual,
          claimed_hash: :crypto.hash(:sha256, k <> v_claimed),
          proof: %{root: root(), nodes: generate_proof_nodes(path, 256)}
        }
    end
  end

  def verify_mismatch(k, v_claimed, proof) do
    calc_claimed = :crypto.hash(:sha256, k <> v_claimed)
    (proof.type == :mismatch) and
    (proof.actual_hash != calc_claimed) and
    (calculate_root(proof.actual_hash, proof.proof.nodes) == proof.proof.root)
  end
  
  # --- PROOF HELPERS (Unified) ---
  defp generate_proof_nodes(path, len) do
    changes_path_from_lcp(path, len - 1, [])
    |> Enum.map(fn {{:n, p, l}, _} ->
      <<_::size(l), my_dir::1, _::bitstring>> = path
      sibling_dir = 1 - my_dir
      
      # Get sibling hash (zero if sparse/collapsed)
      s_hash = get_child_hash(p, l, sibling_dir)
      %{hash: s_hash, direction: sibling_dir, len: l}
    end)
  end

  defp calculate_root(leaf, nodes) do
    Enum.reduce(nodes, leaf, fn %{hash: h, direction: d}, acc ->
      if d == 0, do: :crypto.hash(:sha256, h <> acc), else: :crypto.hash(:sha256, acc <> h)
    end)
  end

  defp find_longest_prefix_node(target) do
    search_key = {:n, target, 256}
    n = closest_or_next_lookup(target, 256)
    p = case :ets.prev_lookup(THUBT, search_key) do
      {k, [{_, h}]} -> {k, h}
      _ -> {nil, nil}
    end

    case {p, n} do
      {{nil, _}, {nil, _}} -> {nil, nil}
      {{nil, _}, n_node}   -> n_node
      {p_node, {nil, _}}   -> p_node
      {{{:n, pp, pl}, _} = p_node, {{:n, np, nl}, _} = n_node} ->
        # Cap LCP at node length to ignore padding
        {_, raw_p} = lcp(target, pp); p_score = min(raw_p, pl)
        {_, raw_n} = lcp(target, np); n_score = min(raw_n, nl)
        if p_score >= n_score, do: p_node, else: n_node
    end
  end

  defp do_divergence_index(<<b::1, r1::bitstring>>, <<b::1, r2::bitstring>>, acc), do: do_divergence_index(r1, r2, acc + 1)
  defp do_divergence_index(_, _, acc), do: acc
end
