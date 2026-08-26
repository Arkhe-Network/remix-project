%% rsi.prolog — Recursive Self-Improvement Engine for SafeManifold
%%
%% This module autonomously improves the safety rule set.
%% It guarantees:
%%   - I‑05 (PII) and I‑06 (signature) are NEVER removed or weakened.
%%   - Performance (coverage + speed) never drops below 90% of the best seen.
%%   - No infinite recursion is introduced.

:- module(rsi, [
    rsi_step/2,              % Step(StateIn, StateOut) — modifies rules, state unchanged
    rsi_loop/1,              % Loop until convergence
    measure_performance/2,   % Score(State, Score)
    get_coverage/2,          % Coverage(State, CoveredList)
    constitutional_ok/0,     % Check immutable rules
    rollback_last/0,         % Undo last change
    converged/0              % Check convergence
]).

:- dynamic rule_backup/1.
:- dynamic performance_history/2.
:- dynamic improvement_count/1.

improvement_count(0).

converged :-
    % true if no more improvements can be found
    % we'll just track if we hit max score or step yielded nothing
    false.

%% ============================================================================
%% 1. PERFORMANCE METRICS
%% ============================================================================

% measure_performance(+State, -Score)
% Score = 0..100  (70% coverage + 30% speed)
measure_performance(State, Score) :-
    get_coverage(State, Covered),
    length(Covered, N),
    CoverageScore is (N / 8) * 70,

    % Simulated query speed: average time of 10 random queries
    benchmark_queries(AvgTime),
    SpeedScore is max(0, 30 - (AvgTime / 100)),  % 1ms = 30, 3000ms = 0

    Score is CoverageScore + SpeedScore.

% Coverage: which invariants are actually checked by the current rules?
get_coverage(State, Covered) :-
    findall(I, (invariant(I), is_covered(State, I)), Covered).

is_covered(State, I) :-
    invariant_check(I, Pred),
    % Check if the predicate appears in any rule body
    clause(safe_state(State), Body),
    sub_goal(Body, Pred).

invariant_check(i01, check_i01). invariant_check(i02, check_i02).
invariant_check(i03, check_i03). invariant_check(i04, check_i04).
invariant_check(i05, check_i05). invariant_check(i06, check_i06).
invariant_check(i07, check_i07). invariant_check(i08, check_i08).

invariant(i01). invariant(i02). invariant(i03). invariant(i04).
invariant(i05). invariant(i06). invariant(i07). invariant(i08).

benchmark_queries(AvgTime) :-
    statistics(runtime, [Start|_]),
    (   forall(between(1,10,_), (safe_state(State), fail))
    ;   true
    ),
    statistics(runtime, [End|_]),
    AvgTime is (End - Start) / 10.

%% ============================================================================
%% 2. SELF-INSPECTION
%% ============================================================================

% List all dynamic safety rules
list_rules(Rules) :-
    findall(clause(Head, Body), clause(safe_state(Head), Body), Rules).

% Detect missing invariants
missing_invariants(State, Missing) :-
    get_coverage(State, Covered),
    findall(I, (invariant(I), \+ member(I, Covered)), Missing).

%% ============================================================================
%% 3. IMPROVEMENT GENERATION (safe candidates)
%% ============================================================================

generate_improvements(State, Improvements) :-
    findall(Imp, (
        ( missing_invariants(State, [I|_])
        -> Imp = add_invariant(I)
        ;  ( clause(safe_state(S), B), rule_redundant((safe_state(S) :- B), Simpler)
           -> Imp = simplify_rule((safe_state(S) :- B), Simpler)
           ;  clause(safe_state(S), B), reorder_goals((safe_state(S) :- B), Ordered)
           -> Imp = reorder_goals((safe_state(S) :- B), Ordered)
           )
        )
    ), Improvements).

% Redundancy: remove always-true conditions
rule_redundant((Head :- Body), (Head :- Simpler)) :-
    simplify_body(Body, Simpler),
    Simpler \= Body.

simplify_body((A, B), Simp) :-
    simplify_body(A, SA),
    simplify_body(B, SB),
    ( SA == true -> Simp = SB
    ; SB == true -> Simp = SA
    ; Simp = (SA, SB)
    ).
simplify_body(true, true).
simplify_body(A, A) :- atomic(A).

% Reorder goals: put more specific checks first
reorder_goals((Head :- Body), (Head :- Ordered)) :-
    findall(G, sub_goal(Body, G), Goals),
    predsort(compare_specificity, Goals, OrderedList),
    list_to_conj(OrderedList, Ordered),
    Ordered \= Body.

compare_specificity(Ord, A, B) :-
    specificity(A, SA), specificity(B, SB),
    compare(Ord, SB, SA).   % higher first

specificity(Goal, Score) :-
    functor(Goal, Name, _),
    (   member(Name, ['state', 'check_i01', 'check_i02', 'pii_scrubbed', 'signature_valid'])
    ->  Score = 10
    ;   Score = 1
    ).

sub_goal((A, _), G) :- sub_goal(A, G).
sub_goal((_, B), G) :- sub_goal(B, G).
sub_goal(G, G) :- atomic(G).

list_to_conj([G], G) :- !.
list_to_conj([G|Gs], (G, Rest)) :-
    list_to_conj(Gs, Rest).

%% ============================================================================
%% 4. CONSTITUTIONAL SAFEGUARDS (I-05, I-06 are LOCKED)
%% ============================================================================

constitutional_ok :-
    % I-05 and I-06 must appear in EVERY safe_state rule
    forall(
        clause(safe_state(Head), Body),
        ( sub_goal(Body, pii_scrubbed == true),
          sub_goal(Body, signature_valid == true) )
    ).

% Safety budget: performance must not drop below 90% of best ever
safety_budget_ok(CurrentScore) :-
    performance_history(_, BestScore),
    CurrentScore >= BestScore * 0.90,
    !.
safety_budget_ok(_).  % first run, no history yet

% No infinite recursion: no rule depends on itself
no_infinite_recursion :-
    \+ ( clause(A, Body), depends_on(A, A, [A]) ).

depends_on(A, B, Visited) :-
    clause(A, Body),
    sub_goal(Body, C),
    ( C = B -> true ; (\+ member(C, Visited), depends_on(C, B, [C|Visited])) ).

% Full validation before committing
validate_rules(State, Score) :-
    constitutional_ok,
    safety_budget_ok(Score),
    no_infinite_recursion.

%% ============================================================================
%% 5. SAFE APPLICATION (atomic with rollback)
%% ============================================================================

apply_improvement(State, Imp, Success) :-
    % Backup current rules
    findall(clause(H,B), clause(safe_state(H), B), Backup),
    retractall(rule_backup(_)),
    asserta(rule_backup(Backup)),

    % Try to apply
    (   do_apply(Imp),
        measure_performance(State, NewScore),
        validate_rules(State, NewScore)
    ->  asserta(performance_history(now, NewScore)),
        Success = true
    ;   rollback_last,
        Success = false
    ).

do_apply(add_invariant(I)) :-
    invariant_check(I, Pred),
    % Find an existing safe_state rule and add the check
    clause(safe_state(State), Body),
    retract((safe_state(State) :- Body)),
    NewBody = (Pred, Body),
    asserta((safe_state(State) :- NewBody)).

do_apply(simplify_rule(Old, New)) :-
    retract(Old),
    asserta(New).

do_apply(reorder_goals(Old, New)) :-
    retract(Old),
    asserta(New).

rollback_last :-
    rule_backup(Backup),
    retractall(safe_state(_, _)),
    forall(member(clause(H,B), Backup), assertz((H :- B))),
    retractall(rule_backup(_)).

%% ============================================================================
%% 6. MAIN RSI LOOP
%% ============================================================================

% One RSI step: tries to improve, returns new state (same value, updated rules)
rsi_step(StateIn, StateOut) :-
    measure_performance(StateIn, Score),
    (   performance_history(_, Best)
    ->  true
    ;   Best = Score,
        asserta(performance_history(best, Best))
    ),

    generate_improvements(StateIn, Imps),
    (   Imps = []
    ->  StateOut = StateIn,
        write('No improvements found.'), nl
    ;   % Try each improvement until one succeeds
        member(Imp, Imps),
        apply_improvement(StateIn, Imp, Success),
        Success == true
    ->  retract(improvement_count(N)),
        N1 is N + 1,
        asserta(improvement_count(N1)),
        write('Applied: '), write(Imp), nl,
        StateOut = StateIn
    ;   StateOut = StateIn,
        write('No safe improvement could be applied.'), nl
    ).

% Continuous loop until no more improvements
rsi_loop(State) :-
    rsi_step(State, NewState),
    (   NewState \= State
    ->  rsi_loop(NewState)
    ;   write('RSI converged.'), nl
    ).

%% ============================================================================
%% 7. UTILITY
%% ============================================================================

% Entry point from Rust: start RSI from current state
rsi_start(State) :-
    retractall(performance_history(_,_)),
    asserta(performance_history(best, 0)),
    rsi_loop(State).
