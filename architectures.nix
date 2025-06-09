[
  # Linux
  { arch = "x86_64"; os = "linux"; target = "x86_64-unknown-linux-gnu"; formats = ["deb" "rpm" "tar.xz"]; }
  { arch = "aarch64"; os = "linux"; target = "aarch64-unknown-linux-gnu"; formats = ["deb" "rpm" "tar.xz"]; }
  { arch = "armv7l"; os = "linux"; target = "armv7-unknown-linux-gnueabihf"; formats = ["tar.xz"]; }
  { arch = "armv6l"; os = "linux"; target = "arm-unknown-linux-gnueabihf"; formats = ["tar.xz"]; }
  # { arch = "riscv32"; os = "linux"; target = "riscv32gc-unknown-linux-gnu"; formats = ["tar.xz"]; }
  # { arch = "riscv64"; os = "linux"; target = "riscv64gc-unknown-linux-gnu"; formats = ["tar.xz"]; }
  # MacOS
  { arch = "x86_64"; os = "macos"; target = "x86_64-apple-darwin"; formats = ["tar.xz"]; }
  { arch = "aarch64"; os = "macos"; target = "aarch64-apple-darwin"; formats = ["tar.xz"]; }
  # Windows
  { arch = "x86_64"; os = "windows"; target = "x86_64-pc-windows-msvc"; formats = ["zip"]; }
  # { arch = "i686"; os = "windows"; target = "i686-pc-windows-msvc"; formats = ["zip"]; }
]
