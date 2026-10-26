class AniCli < Formula
  desc "Terminal client to browse, stream, and download anime"
  homepage "https://github.com/gabrielfranicevich/anime"
  head "https://github.com/gabrielfranicevich/anime.git", branch: "develop"
  license "Unlicense"

  depends_on "rust" => :build
  depends_on "ffmpeg"
  depends_on "mpv" => :recommended
  depends_on "yt-dlp" => :optional
  depends_on "syncplay" => :optional

  def install
    system "cargo", "build", "--release"
    bin.install "target/release/ani-cli"
    man1.install "ani-cli.1"
  end

  test do
    assert_match "ani-cli", shell_output("#{bin}/ani-cli --version")
    assert_match "Usage:", shell_output("#{bin}/ani-cli --help")
  end
end
