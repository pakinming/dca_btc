# just auto-load .env 
set dotenv-load := true

db-create:
    psql -h {{env_var('DB_HOST')}} -U postgres -d postgres -c \
    "CREATE DATABASE {{env_var('DB_NAME')}} WITH LOCALE_PROVIDER=icu ICU_LOCALE='th-TH' ENCODING='UTF8' TEMPLATE=template0;"

# Sync Source Code to Server (rsync)
deploy:
    @echo "Syncing project to {{env_var('HOST_DEPLOY')}}..."
    # Exclude heavy target dir and git metadata
    rsync -avz --exclude 'target' --exclude '.git' ./ {{env_var('HOST_DEPLOY')}}:{{env_var('DEPLOY_PATH')}}/
    @echo "Sync Complete!"
    #ssh {{env_var('HOST_DEPLOY')}} "~/.cargo/bin/cargo build && systemctl restart dca_btc"
    
    # Update service file (scp overwrites automatically)
    scp dca_btc.service {{env_var('HOST_DEPLOY')}}:{{env_var('SERVICE_PATH')}}/

    # Build and Restart to minimize downtime
    ssh {{env_var('HOST_DEPLOY')}} "source /root/.cargo/env && \
        cd {{env_var('DEPLOY_PATH')}} && \
        cargo build --release && \
        systemctl daemon-reload && \
        systemctl restart dca_btc"

    @echo "Deployment Complete! Check status with: ssh {{env_var('HOST_DEPLOY')}} 'systemctl status dca_btc'"
    ssh {{env_var('HOST_DEPLOY')}} "journalctl -u dca_btc -n 10 "


srv-log:
    ssh {{env_var('HOST_DEPLOY')}} "journalctl -u dca_btc.service -f"

srv-stop:
    ssh {{env_var('HOST_DEPLOY')}} "systemctl stop dca_btc.service"

srv-start:
    ssh {{env_var('HOST_DEPLOY')}} "systemctl start dca_btc.service"

srv-restart:
    ssh {{env_var('HOST_DEPLOY')}} "systemctl restart dca_btc.service"