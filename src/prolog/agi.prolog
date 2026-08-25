% ========================================================================
% agi.prolog — ARKHE-χ v8.2 AGI Core (Corrigido)
% ========================================================================

:- module(agi, [
    current_state/1,
    update_state/1,
    evidence/3,
    add_evidence/4,
    passport/5,
    add_passport/5,
    is_audited/1,
    thompson_sample/3,
    update_beta/2,
    breaker_status/1,
    check_action/2,
    invariant/2,
    check_invariants/2,
    edge/4,
    add_edge/4,
    get_agent_context/2,
    validate_suggestion/2,
    help/0
]).

:- use_module(library(apply)).
:- use_module(library(lists)).
:- use_module(library(random)).
:- use_module(library(dcg/basics)).
:- use_module(library(http/json)).
:- use_module(library(date)).

% ========================================================================
% 1. ESTADO DO SISTEMA (thread-safe com mutex)
% ========================================================================

:- dynamic state/8.
:- dynamic state_meta/2.
:- dynamic beta/3.
:- dynamic evidence/3.
:- dynamic passport/5.
:- dynamic edge/4.
:- dynamic breaker_state/1.
:- dynamic next_id/1.
:- dynamic policy/2.

:- mutex_create(state_mutex).

% Estado inicial: 2**64 - 1 como inteiro exato
state(10000, 5, 1000, 512, true, true, 1000, 18446744073709551615).
state_meta(last_update, 0).
breaker_state(closed).
next_id(1).

% Políticas padrão
policy(p01, "ConventionX: evidence required").
policy(p02, "CircuitBreaker: fail-closed").
policy(p03, "Thompson: minimum 0.8 confidence").

current_state(state(TB, AC, SF, EB, PS, SV, RL, MC)) :-
    state(TB, AC, SF, EB, PS, SV, RL, MC).

update_state(NewState) :-
    NewState = state(_, _, _, _, _, _, _, _),
    (   check_invariants(NewState, [])
    ->  with_mutex(state_mutex, (
            retractall(state(_,_,_,_,_,_,_,_)),
            assertz(NewState),
            get_time(Now),
            retractall(state_meta(last_update, _)),
            assertz(state_meta(last_update, Now))
        ))
    ;   check_invariants(NewState, Failed),
        format(atom(Reason), 'Invariant(s) failed: ~w', [Failed]),
        throw(error(invariant_violation, Reason))
    ).

% ========================================================================
% 2. INVARIANTES CONSTITUCIONAIS (I-01 a I-40)
% ========================================================================

invariant(i01, "token_budget >= 0").
invariant(i02, "agent_count <= 10").
invariant(i03, "sandbox_fuel > 0").
invariant(i04, "entropy_bits >= 256").
invariant(i05, "pii_scrubbed == true").
invariant(i06, "signature_valid == true").
invariant(i07, "rate_limit > 0").
invariant(i08, "model_capability >= 2**32").
invariant(i37, "structural_density > 0.2").
invariant(i38, "yield_strength < 836 MPa (0.95 * 880)").
invariant(i39, "fatigue_cycles < 10^7").
invariant(i40, "shear_stress < 5.0 MPa").

check_invariants(State, []) :-
    State = state(TB, AC, SF, EB, PS, SV, RL, MC),
    TB >= 0,
    AC =< 10,
    SF > 0,
    EB >= 256,
    PS == true,
    SV == true,
    RL > 0,
    MC >= 4294967296,  % 2**32
    structural_density(TB, D), D > 0.2,
    yield_strength(AC, Y), Y < 836.0,
    fatigue_cycles(State, F), F < 10000000,
    shear_stress(RL, S), S < 5.0,
    !.
check_invariants(State, Failed) :-
    State = state(_, _, _, _, _, _, _, _),
    findall(Inv, (invariant(Inv, _), \+ check_single(State, Inv)), Failed).

check_single(state(TB, _, _, _, _, _, _, _), i01) :- TB >= 0.
check_single(state(_, AC, _, _, _, _, _, _), i02) :- AC =< 10.
check_single(state(_, _, SF, _, _, _, _, _), i03) :- SF > 0.
check_single(state(_, _, _, EB, _, _, _, _), i04) :- EB >= 256.
check_single(state(_, _, _, _, PS, _, _, _), i05) :- PS == true.
check_single(state(_, _, _, _, _, SV, _, _), i06) :- SV == true.
check_single(state(_, _, _, _, _, _, RL, _), i07) :- RL > 0.
check_single(state(_, _, _, _, _, _, _, MC), i08) :- MC >= 4294967296.
check_single(state(TB, _, _, _, _, _, _, _), i37) :- structural_density(TB, D), D > 0.2.
check_single(state(_, AC, _, _, _, _, _, _), i38) :- yield_strength(AC, Y), Y < 836.0.
check_single(State, i39) :- fatigue_cycles(State, F), F < 10000000.
check_single(state(_, _, _, _, _, _, RL, _), i40) :- shear_stress(RL, S), S < 5.0.

% CORREÇÃO: aritmética com is/2
structural_density(TB, D) :- D is TB / 10000.0.
yield_strength(AC, Y) :- Y is (AC / 10.0) * 880.0.
shear_stress(RL, S) :- S is (RL / 1000.0) * 5.0.
fatigue_cycles(state(_, _, _, _, _, _, _, MC), F) :-
    % Placeholder: em produção, contar ciclos de execução reais
    F is MC mod 10000000.

% ========================================================================
% 3. EVIDÊNCIAS E PASSAPORTES EPISTÊMICOS
% ========================================================================

add_evidence(Id, Content, Status, Metadata) :-
    assertz(evidence(Id, Content, Status)),
    % CORREÇÃO: extrair source/target do Metadata, não átomos literais
    (   is_dict(Metadata)
    ->  get_dict(source, Metadata, Src),
        get_dict(target, Metadata, Tgt),
        get_dict(direction, Metadata, Dir),
        assertz(passport(Id, Src, Tgt, Dir, Metadata))
    ;   assertz(passport(Id, unknown, unknown, unknown, Metadata))
    ).

add_passport(EdgeId, Source, Target, Direction, Metadata) :-
    assertz(passport(EdgeId, Source, Target, Direction, Metadata)).

is_audited(Id) :-
    evidence(Id, _, confirmed),
    passport(Id, _, _, _, Meta),
    is_dict(Meta),
    get_dict(uncertainty, Meta, U),
    number(U),
    U < 0.05.

% ========================================================================
% 4. AMOSTRAGEM DE THOMPSON (Beta-Binomial) — CORRIGIDO
% ========================================================================

init_beta(Action) :-
    (   beta(Action, _, _)
    ->  true
    ;   assertz(beta(Action, 1, 1))
    ).

% CORREÇÃO: random_beta usando distribuição Beta via gama
random_beta(Alpha, Beta, X) :-
    Alpha > 0, Beta > 0,
    random_gamma(Alpha, G1),
    random_gamma(Beta, G2),
    Sum is G1 + G2,
    Sum > 0,
    X is G1 / Sum.

% CORREÇÃO: Marsaglia & Tsang para Shape > 1
random_gamma(Shape, Val) :-
    Shape > 1,
    !,
    D is Shape - 1/3,
    C is 1 / sqrt(9*D),
    repeat,
    random(U1), random(U2),
    V0 is 1 + C * (U1 * 2 - 1),  % Z ~ Uniform(-1,1) transformado
    V is V0^3,
    (   V > 0,
        U2 < 1 - 0.0331 * (U1 * 2 - 1)^4,
        Val is D * V,
        !
    ;   fail
    ).
random_gamma(Shape, Val) :-
    Shape =< 1,
    !,
    random_gamma(Shape + 1, G),
    random(U),
    Val is G * U^(1/Shape).

% CORREÇÃO: max_member com comparação numérica correta
thompson_sample(Actions, BestAction, Confidence) :-
    maplist(init_beta, Actions),
    findall(Conf-Action, (
        member(Action, Actions),
        beta(Action, S, F),
        random_beta(S, F, Conf)
    ), Pairs),
    (   Pairs = []
    ->  BestAction = none, Confidence = 0.0
    ;   max_beta_pair(Pairs, Confidence-BestAction)
    ).

max_beta_pair([P], P) :- !.
max_beta_pair([C1-A1, C2-A2 | Rest], Best) :-
    (   C1 >= C2
    ->  max_beta_pair([C1-A1 | Rest], Best)
    ;   max_beta_pair([C2-A2 | Rest], Best)
    ).

update_beta(Action, Success) :-
    with_mutex(state_mutex, (
        retract(beta(Action, S, F)),
        (   Success == true
        ->  S1 is S + 1
        ;   F1 is F + 1
        ),
        assertz(beta(Action, S1, F1))
    )).

% ========================================================================
% 5. CIRCUITBREAKER
% ========================================================================

breaker_status(Status) :-
    breaker_state(Status).

check_action(Action, Result) :-
    current_state(State),
    (   check_invariants(State, []),
        breaker_state(closed)
    ->  Result = ok(Action)
    ;   breaker_state(open)
    ->  Result = blocked(reason("Breaker is open"))
    ;   Result = blocked(reason("Invariant violation"))
    ).

open_breaker :-
    with_mutex(state_mutex, (
        retractall(breaker_state(_)),
        assertz(breaker_state(open))
    )),
    log("Breaker opened due to invariant violation.").

close_breaker :-
    with_mutex(state_mutex, (
        retractall(breaker_state(_)),
        assertz(breaker_state(closed))
    )),
    log("Breaker closed manually.").

% ========================================================================
% 6. GRAFO DE CONHECIMENTO (SCGG) COM PASSAPORTES
% ========================================================================

% CORREÇÃO: aridade consistente (4, não 5)
add_edge(From, To, Label, Passport) :-
    gen_id(EdgeId),
    assertz(edge(From, To, Label, EdgeId)),
    (   is_dict(Passport)
    ->  get_dict(source, Passport, Src),
        get_dict(target, Passport, Tgt),
        get_dict(direction, Passport, Dir),
        assertz(passport(EdgeId, Src, Tgt, Dir, Passport))
    ;   assertz(passport(EdgeId, From, To, forward, Passport))
    ).

% CORREÇÃO: ID único com contador atômico
gen_id(Id) :-
    with_mutex(state_mutex, (
        retract(next_id(N)),
        Id is N,
        N1 is N + 1,
        assertz(next_id(N1))
    )).

audited_edge(From, To, Label) :-
    edge(From, To, Label, EdgeId),
    is_audited(EdgeId).

% ========================================================================
% 7. SAFELSP — CONTEXTUALIZAÇÃO PARA AGENTES LLM
% ========================================================================

get_agent_context(AgentId, Context) :-
    current_state(State),
    findall(Inv, invariant(Inv, _), Invariants),
    findall(Pol, policy(_, Pol), Policies),
    breaker_status(BStatus),
    Context = json{
        agent_id: AgentId,
        state: State,
        invariants: Invariants,
        policies: Policies,
        breaker: BStatus
    }.

validate_suggestion(Suggestion, Result) :-
    is_dict(Suggestion),
    get_dict(evidence, Suggestion, Ev),
    get_dict(confidence, Suggestion, Conf),
    is_list(Ev),
    length(Ev, N),
    N > 0,
    number(Conf),
    Conf >= 0.8,
    !,
    Result = ok.
validate_suggestion(_, rejected(reason("Evidence or confidence insufficient"))).

% ========================================================================
% 8. LOGGING E MONITORAMENTO
% ========================================================================

log(Message) :-
    get_time(Now),
    format_time(string(TimeStr), '%Y-%m-%d %H:%M:%S', Now),
    format('~w [AGI] ~w~n', [TimeStr, Message]).

% ========================================================================
% 9. TESTES UNITÁRIOS
% ========================================================================

:- begin_tests(agi).

test(invariant_i01_pass) :-
    check_single(state(100, 1, 1, 256, true, true, 1, 4294967296), i01).

test(invariant_i01_fail) :-
    \+ check_single(state(-1, 1, 1, 256, true, true, 1, 4294967296), i01).

test(structural_density) :-
    structural_density(5000, D),
    D =:= 0.5.

test(yield_strength) :-
    yield_strength(5, Y),
    Y =:= 440.0.

test(thompson_basic) :-
    retractall(beta(_,_,_)),
    thompson_sample([a, b], _, Conf),
    Conf > 0,
    Conf =< 1.

test(breaker_cycle) :-
    close_breaker,
    breaker_status(closed),
    open_breaker,
    breaker_status(open),
    close_breaker.

test(passport_creation) :-
    retractall(agi:passport(_,_,_,_,_)),
    add_passport(e1, src, tgt, forward, json{uncertainty:0.01}),
    passport(e1, src, tgt, forward, _).

:- end_tests(agi).

% ========================================================================
% 10. INICIALIZAÇÃO
% ========================================================================

:- initialization
    format('AGI Core v8.2 loaded. Run ?- run_tests(agi).~n').

help :-
    format('Available predicates:~n'),
    format('  update_state(+State)~n'),
    format('  thompson_sample(+Actions, -Choice, -Confidence)~n'),
    format('  add_evidence(+Id, +Content, +Status, +Metadata)~n'),
    format('  check_action(+Action, -Result)~n'),
    format('  get_agent_context(+AgentId, -Context)~n'),
    format('  run_tests(agi).~n').
