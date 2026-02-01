# just auto-load .env 
set dotenv-load := true

db-create:
    psql -h {{env_var('DB_HOST')}} -U postgres -d postgres -c \
    "CREATE DATABASE dca_btc WITH LOCALE_PROVIDER=icu ICU_LOCALE='th-TH' ENCODING='UTF8' TEMPLATE=template0;"

# Build for Linux (using cross for macOS -> Linux)
build-linux:
    @echo "Building for Linux (x86_64-unknown-linux-musl)..."
    # cross build --target x86_64-unknown-linux-musl --release
    docker run --rm -v "${PWD}:/volume" --workdir /volume clux/muslrust cargo build --release

# Deploy to Server
# Usage: just deploy <user@host>
deploy host: build-linux
    @echo "Deploying to {{host}}..."
    # Create directory if not exists
    ssh {{host}} "mkdir -p /opt/dca_btc"
    # Copy binary
    scp target/aarch64-unknown-linux-musl/release/dca_btc .env {{host}}:/opt/dca_btc/
    # Copy systemd service file
    scp dca_btc.service {{host}}:/etc/systemd/system/
    # Reload daemon and restart service
    ssh {{host}} "systemctl daemon-reload && systemctl enable dca_btc && systemctl restart dca_btc"
    @echo "Deployment Complete! Check status with: ssh {{host}} 'systemctl status dca_btc'"

# Sync Source Code to Server (rsync)
sync host:
    @echo "Syncing project to {{host}}..."
    # Exclude heavy target dir and git metadata
    rsync -avz --exclude 'target' --exclude '.git' ./ {{host}}:/opt/dca_btc/
    @echo "Sync Complete!"
    # ssh {{host}} "~/.cargo/bin/cargo build && systemctl restart dca_btc"
    ssh {{host}} "source /root/.cargo/env && cargo build --release && systemctl restart dca_btc"
