defmodule VecPakEx.MixProject do
  use Mix.Project

  def project do
    [
      app: :vecpak_ex_nif,
      version: "0.1.0",
      elixir: "~> 1.19",
      build_embedded: Mix.env() == :prod,
      start_permanent: Mix.env() == :prod,
      description: "VecPak in RUST for Elixir",
      deps: deps(),
    ]
  end

  def application do
    [
    ]
  end

  defp deps do
    [
      {:rustler, ">= 0.36.1", runtime: false, optional: true},
    ]
  end
end
