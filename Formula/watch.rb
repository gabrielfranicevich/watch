class Watch < Formula
  desc "Terminal client to browse, stream, and download anime"
  homepage "https://github.com/gabrielfranicevich/watch"
  version "0.1.0"
  url "https://github.com/gabrielfranicevich/watch.git", using: :git, revision: "dbed6ccc2c8629bc48e74c8e918f40bc7c8e5358"
  license "GPL-3.0-only"

  depends_on "rust" => :build
  depends_on "ffmpeg"
  depends_on "mpv" => :recommended
  depends_on "yt-dlp" => :optional
  depends_on "syncplay" => :optional

  def install
    system "cargo", "build", "--locked", "--release", "--bin", "watch"
    bin.install "target/release/watch"
    man1.install "watch.1"
  end

  test do
    assert_match "watch", shell_output("#{bin}/watch --version")
    assert_match "Usage:", shell_output("#{bin}/watch --help")
  end
end
