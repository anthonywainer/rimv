# Observation, evidence and UX metrics

## Record facts before interpretations

An observation note should include session ID, task ID, build, platform, starting state, timestamp/step, action, observed UI response, exact error and outcome. Separate:

- **Observed:** “Participant opened Settings twice; capture remained idle.”
- **Verbatim feedback:** “I expected it to start immediately.” (only if actually said and permitted to quote)
- **Interpretation:** “Recording feedback may be insufficient.”
- **Hypothesis:** “A persistent labeled live indicator might help.”

Label unobserved or hypothetical cases as **simulation/walkthrough**, not user testing.

## Task outcomes

Use consistent predeclared categories: `independent success`, `assisted success`, `partial`, `failure`, `abandoned`, `not attempted`. Define what counts as success for each task. Record moderator assistance explicitly. A user who reaches the goal only after receiving a hint is not an independent success.

## Recommended metrics

| Metric | Use | Caution |
|---|---|---|
| Independent completion | Can participants accomplish the goal? | Always give numerator and denominator |
| Assistance count/type | Where does UI fail to support self-service? | Define what counts as a hint |
| Critical error | Safety, loss, privacy or inability to stop capture | Record exact failure and reproduction context |
| Time on task | Narrow comparative/benchmark task | Think-aloud and interventions change timing |
| Wrong turns/recovery | Locates confusion | Agree on definition beforehand |
| Confidence or ease | Short post-task participant self-report | Do not treat opinion as behavioral evidence |
| Success under interruption | Measures resilience | State controlled interruption conditions |

`Independent completion = number independently successful / number who attempted that task.` Exclude not-attempted tasks from that denominator and show missing data. If using time, state whether failures or assisted completions are included.

## Interpreting small studies

With a small formative sample, counts are descriptive of **those participants and that setup**. Do not extrapolate to all RimV users or assert precise population percentages. Use repeated observations and clear failure traces to prioritize a retest, not to manufacture certainty. If tasks or versions change between sessions, do not combine them into a benchmark denominator without an explicit reason.

## Metrics are not the goal

Observed reasons for confusion, task failures, recovery difficulties and privacy misunderstandings usually inform concrete design changes more directly than a single “UX score.” Never reduce accessibility or capture safety to an aggregate metric that can hide a critical problem.
