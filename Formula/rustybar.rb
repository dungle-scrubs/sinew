class Rustybar < Formula
  desc "macOS menu bar replacement with notch-aware layouts and hot-reload config"
  homepage "https://github.com/dungle-scrubs/rustybar"
  url "https://github.com/dungle-scrubs/rustybar/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "fba62efbf14f57455aad2aaad9d5cdf7ecb1f758bae663239da11cb1a369f461"
  license "MIT"

  depends_on :macos

  def install
    system "cargo", "build", "--release"
    bin.install "target/release/rustybar"
    bin.install "target/release/rustybar-msg" if File.exist?("target/release/rustybar-msg")
  end

  service do
    run [opt_bin/"rustybar"]
    keep_alive true
    log_path var/"log/rustybar.log"
    error_log_path var/"log/rustybar.err"
    environment_variables RUST_LOG: "info"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/rustybar --version")
  end
end
