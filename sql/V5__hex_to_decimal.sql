-- https://stackoverflow.com/questions/37248518/sql-function-to-convert-numeric-to-bytea-and-bytea-to-numeric

CREATE OR REPLACE FUNCTION bytea2numeric(_b BYTEA) RETURNS NUMERIC AS $$
DECLARE
    _n NUMERIC := 0;
BEGIN
    FOR _i IN 0 .. LENGTH(_b)-1 LOOP
        _n := _n*256+GET_BYTE(_b,_i);
    END LOOP;
    RETURN _n;
END;
$$ LANGUAGE PLPGSQL IMMUTABLE STRICT;

CREATE OR REPLACE FUNCTION numeric2bytea(_n NUMERIC) RETURNS BYTEA AS $$
DECLARE
    _b BYTEA := '\x';
    _v INTEGER;
BEGIN
    WHILE _n > 0 LOOP
        _v := _n % 256;
        _b := SET_BYTE(('\x00' || _b),0,_v);
        _n := (_n-_v)/256;
    END LOOP;
    RETURN _b;
END;
$$ LANGUAGE PLPGSQL IMMUTABLE STRICT;
