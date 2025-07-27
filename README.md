# Autoclicker

[![build status](https://codeberg.org/Land/autoclicker/actions/workflows/build.yaml/badge.svg)](https://codeberg.org/Land/autoclicker/actions?workflow=build.yaml)

**A modern Linux autoclicker with a GTK UI and global shortcut support that even works in Wayland.**
Inspired by [ydotool](https://github.com/ReimuNotMoe/ydotool).

<details>
  <summary><b>Click me for screenshots!</b></summary>

  ![Mouse](assets/screenshots/mouse.png?raw=true "Mouse")

  ![Keyboard](assets/screenshots/keyboard.png?raw=true "Keyboard")

  ![Keyboard editor](assets/screenshots/key_editor.png?raw=true "Keyboard editor")
</details>

## ✨ Features

- Fully-featured GUI using GTK4
- Made with Rust
- Customizable key sequences and timings
- Global keyboard shortcut support via XDG portals
- Supports both keyboard and mouse automation
- Built specifically for Linux (Wayland and X11 compatible)

## 📦 Installation
### 🧪 Arch-based distros (via my repo)
```sh
# Install pacsync command
sudo pacman -S --needed pacutils

# Add repo
echo "[landware]              
Server = https://repo.kage.sj.strangled.net/landware/x86_64
SigLevel = DatabaseNever PackageNever TrustedOnly" | sudo tee -a /etc/pacman.conf

# Sync the repo
sudo pacsync landware

# Install like a normal package
sudo pacman -S autoclicker-git
```

### 🔧 Building
```sh
# Install deps
# Arch Linux
sudo pacman -S --needed slurp cairo gtk4 libadwaita polkit \
	hicolor-icon-theme glib2 glibc libevdev \
	git rust sed libgit2

# Clone the repo
git clone https://codeberg.org/Land/autoclicker.git
cd autoclicker

# Build the GUI
cargo b --release --package autoclicker

# Build the background daemon
cargo b --release --package autoclickerd

# Install binaries
sudo install -Dm755 "target/release/autoclicker" "/usr/bin/autoclicker"
sudo install -Dm755 "target/release/autoclickerd" "/usr/bin/autoclickerd"

# Install license file, icon, and desktop file
sudo install -Dm644 "LICENSE" -t "/usr/share/licenses/autoclicker/"
sudo install -Dm644 "assets/icon.svg" -T "/usr/share/icons/hicolor/scalable/apps/dev.land.Autoclicker.svg"
sudo install -Dm644 "assets/dev.land.Autoclicker.desktop" -t "/usr/share/applications/"

# install systemd service
sudo install -Dm644 "assets/autoclickerd.service" -t "/usr/lib/systemd/user/"

# Optional cleanup
sudo pacman -Rs git rust sed libgit2
```

## 🛠️ Usage
1. Launch the GUI from your app launcher or by running:
  ```sh
  autoclicker
  ```
2. Customize the timing, click type, etc.
3. Define a global shortcut through your system's shortcut manager (using the XDG portal)
4. Enjoy!

Configuration
-------------

A configuration file with all defaults is created upon first launch in `$XDG_CONFIG_HOME/cava/config` or `$HOME/.config/cava/config`. Below is a breakdown of each option available.

### `[general]`
|Option|Description|
|------|-----------|
| `socket_path` | Path to the unix socket used for communication between the daemon and client. `$id` will be replaced by the current UID. |

### `[client]`
|Option|Description|
|------|-----------|
| `disable_window_controls` | Disable the minimize/close buttons in the UI. Useful for Hyprland-like setups |

### `[daemon]`
|Option|Description|
|------|-----------|
| `hyprland_ipc` | Refer to [Hyprland](#hyprland) |
| `dry_run` | If `true`, the daemon accepts all requests without actually acting on them. Useful for testing. |

### `[daemon.mouse]`
|Option|Description|
|------|-----------|
| `disabled` | Disable all mouse automation. |
| `added_delay` | Additional delay added by the daemon for mouse actions; on top of the delay set by the UI. |

### `[daemon.keyboard]`
|Option|Description|
|------|-----------|
| `disabled` | Disable all keyboard automation. |
| `added_delay` | Additional delay added by the daemon for keyboard actions; on top of the delay set by the UI. |

## 🗒️ Notes
- The background daemon (`autoclickerd`) runs in user space and is required for listening to global hotkeys and handling low-level input events.
- On first activation of the autoclicker, the GUI with prompt you to enable to daemon if it cannot be detected. If you want to start the daemon, as well as setting it to start on boot, without the GUI, run:
  ```sh
  systemctl --user enable --now autoclickerd.service
  ```
- If a `systemctl` error is happening when starting the daemon, whether it be from the console or GUI, make sure `ls -l /dev/uinput` displays the correct permissions and group:
  ```sh
  crw-rw----+ 1 root input 10, 223 Jul 23 01:28 /dev/uinput
  ```
  If they are wrong:
  ```sh
  echo 'KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"' | sudo tee /etc/udev/rules.d/99-uinput.rules
  
  # Restart udev, or reboot
  sudo udevadm control --reload
  sudo udevadm trigger
  ```

### Hyprland
Because of Wayland limitations, if you have multiple monitors the cursor **might not move to the correct location for every click.** If you use Hyprland, there is a method implemented that fixes this, although it makes clicks ~6ms slower. You can turn this off in the config.