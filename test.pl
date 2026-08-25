:- use_module('src/prolog/agi.prolog').
:- initialization(main, main).
main :-
    retractall(agi:passport(_,_,_,_,_)),
    agi:add_passport(e1, src, tgt, forward, json{uncertainty:0.01}),
    (agi:passport(e1, src, tgt, forward, _) -> writeln(ok) ; writeln(fail)).
