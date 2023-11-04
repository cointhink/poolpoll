CREATE OR REPLACE FUNCTION hex_to_decimal3(hex_string text)
 RETURNS numeric
 LANGUAGE plpgsql
 IMMUTABLE
AS $function$
declare
    hex_string_lower text := lower(hex_string);
    i int;
    digit int;
    s numeric := 0;
begin
    for i in 1 .. length(hex_string) loop
        digit := position(substr(hex_string_lower, i, 1) in '0123456789abcdef') - 1;
        if digit < 0 then
            raise '"%" is not a valid hexadecimal digit', substr(hex_string_lower, i, 1) using errcode = '22P02'; 
        end if;
        s := s * 16 + digit;
    end loop;
   
    return s;
end
$function$;


CREATE OR REPLACE FUNCTION string_nchars(text, integer) RETURNS setof text AS $$
SELECT substring($1 from n for $2) FROM generate_series(1, length($1), $2) n;
$$ LANGUAGE sql IMMUTABLE;


