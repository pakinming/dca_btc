# just auto-load .env 
set dotenv-load := true

db-create:
    psql -h {{env_var('DB_HOST')}} -U postgres -d postgres -c \
    "CREATE DATABASE dca_btc WITH LOCALE_PROVIDER=icu ICU_LOCALE='th-TH' ENCODING='UTF8' TEMPLATE=template0;"

# Sync Source Code to Server (rsync)
deploy host:
    @echo "Syncing project to {{host}}..."
    # Exclude heavy target dir and git metadata
    rsync -avz --exclude 'target' --exclude '.git' ./ {{host}}:{{env_var('DEPLOY_PATH')}}/
    @echo "Sync Complete!"
    #ssh {{host}} "~/.cargo/bin/cargo build && systemctl restart dca_btc"
    
    # Update service file (scp overwrites automatically)
    scp dca_btc.service {{host}}:{{env_var('SERVICE_PATH')}}/

    # Build and Restart to minimize downtime
    ssh {{host}} "source /root/.cargo/env && \
        cd {{env_var('DEPLOY_PATH')}} && \
        cargo build --release && \
        systemctl daemon-reload && \
        systemctl restart dca_btc"

    @echo "Deployment Complete! Check status with: ssh {{host}} 'systemctl status dca_btc'"
    ssh {{host}} "journalctl -u dca_btc -n 10 "
