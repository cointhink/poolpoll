CREATE TABLE pools (
    contract_address VARCHAR(40) PRIMARY KEY,
    token0 VARCHAR(40),
    token1 VARCHAR(40)
);

CREATE TABLE reserves (
    contract_address VARCHAR(40),
    block_number INTEGER,
    x VARCHAR(78),
    y VARCHAR(78),
    unique (contract_address, block_number)

/*
    x NUMERIC(78),
    y NUMERIC(78)
*/
);

CREATE TABLE coins (
    contract_address VARCHAR(40) PRIMARY KEY,
    name TEXT,
    symbol TEXT,
    decimals INTEGER
);

