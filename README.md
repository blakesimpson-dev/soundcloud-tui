# soundcloud-tui

A terminal SoundCloud client written in Rust.

## Prerequisites

```bash
sudo pacman -S mpv
```

## Installation

Clone the repository and install:

```bash
git clone https://github.com/opensoundcloud/soundcloud-tui.git
cd soundcloud-tui
cargo install --path .
```

Ensure `~/.cargo/bin` is in your `$PATH`.

## Usage

Launch from any terminal:

```bash
soundcloud-tui
```

### Controls

- `Space`: Play / Pause
- `n`: Skip to Next track
- `b`: Skip to Previous track
- `a`: Add highlighted track to Queue
- `Enter`: Play selected track / Select artist
- `/`: Search tracks
- `u`: Load user profile / likes
- `j` / `k` (or `Up` / `Down`): Navigate list
- `h` / `l` (or `Left` / `Right`): Switch panes
- `Tab` / `>` / `<`: Switch views (Now Playing, Lyrics, Queue, Tracks)
- `t`: Open theme picker
- `f`: Open filter / sort modal
- `+` / `-`: Volume up / down
- `q`: Quit

## Credits

Special thanks to [HaseebKhalid1507/Myx](https://github.com/HaseebKhalid1507/Myx) for inspiration!

## Uninstall

```bash
cargo uninstall soundcloud-tui
```
