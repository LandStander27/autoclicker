# Autoclicker
- An autoclicker for Linux that works similarly to [ydotool](https://github.com/ReimuNotMoe/ydotool), but with a UI + global shortcut

![Screenshot of UI](assets/ui.png?raw=true "Screenshot of UI")

## Usage
### Using my repo (For Arch-based distros)
```sh
# Install pacsync command
sudo pacman -S --needed pacutils

# Add repo
echo "[landware]              
Server = https://repo.kage.sj.strangled.net/landware/x86_64
SigLevel = DatabaseNever PackageNever TrustedOnly" | sudo tee -a /etc/pacman.conf

# Sync repo without syncing all repos
sudo pacsync landware

# Install like a normal package
sudo pacman -S autoclicker-git
```

### Building
```sh
# Install deps
# Arch Linux
pacman -S --needed slurp cairo gtk4 libadwaita polkit hicolor-icon-theme glib2 glibc libevdev git rust sed libgit2

# Clone the repo
git clone https://codeberg.org/Land/autoclicker.git
cd autoclicker

# Build the GUI
cargo b --release --package autoclicker

# Build the background daemon
cargo b --release --package autoclickerd

# Install binaries
install -Dm755 "target/release/autoclicker" "/usr/bin/autoclicker"
install -Dm755 "target/release/autoclickerd" "/usr/bin/autoclickerd"

# Install license file, icon, and desktop file
install -Dm644 "LICENSE" -t "/usr/share/licenses/autoclicker/"
install -Dm644 "assets/icon.svg" -T "/usr/share/icons/hicolor/scalable/apps/dev.land.Autoclicker.svg"
install -Dm644 "assets/dev.land.Autoclicker.desktop" -t "/usr/share/applications/"

# install systemd service
install -Dm644 "assets/autoclickerd.service" -t "/usr/lib/systemd/user/"
```