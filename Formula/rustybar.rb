class Rustybar < Formula
  desc "macOS menu bar replacement with notch-aware layouts and hot-reload config"
  homepage "https://github.com/dungle-scrubs/rustybar"
  url "https://github.com/dungle-scrubs/rustybar/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER"
  license "MIT"

  depends_on :macos

  def install
    system "cargo", "build", "--release", "--locked"
    bin.install "target/release/rustybar"
    bin.install "target/release/rustybar-msg" if File.exist?("target/release/rustybar-msg")
  end

  test do
    assert_match "rustybar", shell_output("#{bin}/rustybar --help 2>&1", 1)
  end
end
