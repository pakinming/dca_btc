CREATE TABLE IF NOT EXISTS trades (
    id SERIAL PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    amount_thb INT NOT NULL,
    rat INT NOT NULL,
    type VARCHAR(10) NOT NULL,
    timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    response_json JSONB
);
