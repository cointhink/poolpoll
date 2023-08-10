CREATE TABLE pools (
    index INTEGER PRIMARY KEY,
    contract_address VARCHAR(40),
    token0 VARCHAR(40),
    token1 VARCHAR(40)
);

CREATE TABLE reserves (
    pool_index INTEGER,
    block_number INTEGER,
    x VARCHAR(78),
    y VARCHAR(78)

/*
    x NUMERIC(78),
    y NUMERIC(78)
*/
);

