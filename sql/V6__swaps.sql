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

CREATE OR REPLACE FUNCTION log_to_swap() RETURNS TRIGGER AS $log_swap$
    BEGIN
        IF NEW.topic0 = 'd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822' THEN -- uniswap v2 swap()
        INSERT INTO swaps SELECT NEW.address, NEW.block_number, NEW.block_hash, NEW.transaction_index, NEW.transaction_hash, in0(NEW), in1(NEW), out0(NEW), out1(NEW);
        END IF;
        RETURN NULL; -- result is ignored since this is an AFTER trigger
    END;
$log_swap$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER log_swap
AFTER INSERT ON logs
    FOR EACH ROW EXECUTE FUNCTION log_to_swap();

