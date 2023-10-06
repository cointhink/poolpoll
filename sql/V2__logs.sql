CREATE TABLE logs (
  address VARCHAR(40),
  block_hash VARCHAR(64),
  block_number Int4,
  transaction_index Int4,
  transaction_hash VARCHAR(64),
  value NUMERIC(78),
  topic0 VARCHAR(64),
  topic1 VARCHAR(64),
  topic2 VARCHAR(64)
);
