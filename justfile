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

# Sync Source Code to Server (rsync)
deploy host:
    @echo "Syncing project to {{host}}..."
    # Exclude heavy target dir and git metadata
    rsync -avz --exclude 'target' --exclude '.git' ./ {{host}}:/opt/dca_btc/
    @echo "Sync Complete!"
    #ssh {{host}} "~/.cargo/bin/cargo build && systemctl restart dca_btc"
    
    # Update service file (scp overwrites automatically)
    scp dca_btc.service {{host}}:/etc/systemd/system/

    # Build and Restart to minimize downtime
    ssh {{host}} "source /root/.cargo/env && \
        cd /opt/dca_btc && \
        cargo build --release && \
        systemctl daemon-reload && \
        systemctl restart dca_btc"

    @echo "Deployment Complete! Check status with: ssh {{host}} 'systemctl status dca_btc'"
    ssh {{host}} "journalctl -u dca_btc -n 10 "
