defmodule VecPak do
  use Rustler,
    otp_app: :vecpak_ex_nif,
    crate: :vecpak_ex_nif

  def encode(_term), do: :erlang.nif_error(:nif_not_loaded)
  def decode(_binary), do: :erlang.nif_error(:nif_not_loaded)
end
