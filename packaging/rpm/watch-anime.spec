Name:           watch-anime
Version:        0.1.0
Release:        1%{?dist}
Summary:        Terminal client to browse, stream, and download anime
License:        GPL-3.0-only
URL:            https://github.com/gabrielfranicevich/watch
Source0:        https://github.com/gabrielfranicevich/watch/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  gcc
BuildRequires:  cargo
BuildRequires:  rust
Requires:       ffmpeg
Recommends:     mpv
Suggests:       yt-dlp
Suggests:       syncplay

%description
watch searches enabled anime providers, groups matching titles by
audio/subtitle language, and delegates playback or downloading to
external media tools. It does not host media.

%prep
%autosetup -n watch-%{version}

%build
cargo build --locked --release --bin watch

%check
cargo test --locked --all-targets

%install
install -D -m 0755 target/release/watch %{buildroot}%{_bindir}/watch-anime
install -D -m 0644 watch.1 %{buildroot}%{_mandir}/man1/watch-anime.1

%files
%{_bindir}/watch-anime
%{_mandir}/man1/watch-anime.1*
%license LICENSE
%doc README.md
