# Planning a RimV usability study

## Start with the decision

Write one sentence: “We need to decide whether [flow] works for [user] on [platform] under [conditions].” Examples: Can a first-time macOS user understand microphone permissions? Can a Windows user stop an active recording? Can a web user distinguish partial and final transcript segments?

Define what the test will *not* answer. Testing model accuracy, DSP latency, WCAG conformance, and visual preference are separate questions unless deliberately included.

## Choose a method

- **Formative moderated:** Observe realistic tasks, usually with think-aloud. Best for finding and understanding problems; change the prototype between rounds, but label versions separately.
- **Comparative:** Compare two implementations using the same task/scenario and balanced order when feasible. Define the comparison before sessions.
- **Benchmark:** Keep tasks, environment, criteria and measurement consistent across rounds. More participants and statistical planning may be needed to generalize results.
- **Expert walkthrough:** Useful before recruitment; do not report as participant evidence.
- **Accessibility user session:** Include people who use relevant assistive technologies; complement, never replace, technical accessibility evaluation.

## Recruit for real tasks

Recruit people who would plausibly use RimV, not only team members. Segment when behavior genuinely differs: first-time vs experienced transcription users, platform, keyboard use, assistive technology, or use of local models. Record recruitment criteria, actual session counts and limitations.

For a focused formative round, **roughly 5–8 representative people per materially distinct user group** is a practical starting point, not a guarantee of discovering a fixed percentage of problems. A pilot is not a participant result unless predeclared as such. Larger samples are appropriate for diverse audiences, rare issues, accessibility variety, or reliable quantitative estimates.

## Create task scenarios

Describe an outcome in everyday language. Do not tell participants exactly which control to select or expose the feature being evaluated. Define independently observable success, permitted assistance, starting state, task cutoff, and realistic error paths. Pilot with someone outside the authoring process; fix confusing task wording before main sessions.

Good: “You want to dictate a short note using your external microphone, then copy the final text.”

Leading: “Click the microphone dropdown, select USB Mic, press Start, and copy the transcript.”

## Fix the conditions

Record app build/commit, target OS and version, device/audio stack, screen size, input/assistive technology, model availability, permission starting state, test fixture and network condition where relevant. Do not compare tasks across materially different setups without noting the difference.

## Predefine outcomes

For each task decide before testing:

- independently successful / successful with assistance / partial / unsuccessful / abandoned / not attempted;
- any safety or privacy-critical success criteria;
- when timing begins/ends **if** timing is meaningful;
- what qualifies as moderator assistance;
- required observation fields.

Do not interpret five qualitative sessions as statistically representative. Report counts alongside denominators and exact tested conditions.

## Readiness gate

Before the first participant: consent text ready; recording permissions handled; fake or sanitized audio available; core app stable enough for the task; moderator script and note-taking roles assigned; tasks piloted; withdrawal/deletion procedure defined. If a task could expose private data, change the environment or do not conduct it.
