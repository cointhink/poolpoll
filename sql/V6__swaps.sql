CREATE TABLE IF NOT EXISTS swaps (
  pool_contract_address VARCHAR(40),
  block_number Int4,
  block_hash VARCHAR(64),
  transaction_index Int4,
  transaction_hash VARCHAR(64),
  in0 DECIMAL,
  in0_eth DECIMAL,
  in1 DECIMAL,
  in1_eth DECIMAL,
  out0 DECIMAL,
  out1 DECIMAL
);

create index IF NOT EXISTS swaps_block_number on swaps (block_number);

