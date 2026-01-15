sudo dnf config-manager --set-enabled crb
sudo dnf install -y openssl-devel pkg-config cmake unixODBC-devel

if ! command -v rustc &> /dev/null; then
    curl https://sh.rustup.rs -sSf | sh
fi
