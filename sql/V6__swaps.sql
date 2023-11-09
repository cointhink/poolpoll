CREATE TABLE IF NOT EXISTS swaps (
  address VARCHAR(40),
  block_number Int4,
  block_hash VARCHAR(64),
  transaction_index Int4,
  transaction_hash VARCHAR(64),
  in0 DECIMAL,
  in1 DECIMAL,
  out0 DECIMAL,
  out1 DECIMAL
);

