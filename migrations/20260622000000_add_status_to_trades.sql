ALTER TABLE trades ADD COLUMN status VARCHAR(20) DEFAULT 'open';

UPDATE trades SET status = 'filled' WHERE receive_amount > 0;
