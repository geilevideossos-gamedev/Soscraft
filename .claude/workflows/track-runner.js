export const meta = {
  name: 'track-runner',
  description: 'Run one round track in its own worktree: commit groups in order, adversarial review, clippy and tests of touched crates',
  whenToUse: 'args: {track, worktree, branch, target, base, briefs, groups:[{key,prompt}], round?, avoid?, notes?, skipReview?}. See docs/PROGRESS.md "How to continue".',
  phases: [
    { title: 'Build', detail: 'one agent per commit group, in order' },
    { title: 'Review', detail: 'adversarial review of the whole track, then fixes' },
    { title: 'Verify', detail: 'clippy and tests of the touched crates, combined kit line' },
  ],
}

const A = args
const LOG = `${A.worktree}\\docs\\round${A.round || 8}\\track-${A.track}-log.md`
const RULES = `You work on track ${String(A.track).toUpperCase()} of the fork described in CLAUDE.md (already in your context; follow it strictly).
WORKTREE: ${A.worktree} (git worktree on branch ${A.branch}). Do ALL file edits and git commands inside this worktree only, using absolute paths under it. Never write into C:\\Dev\\Soscraft\\mashup-crafting or another worktree, not even a scratch file. Never push, merge or rebase.
CARGO: only through the wrapper, from PowerShell, with this track's own target dir, from the worktree: \`Set-Location ${A.worktree}; & .\\scripts\\cargo-slot.ps1 +target=${A.target} <subcommand> <args>\`. It adds -j 8 and waits for one of two machine-wide build slots (other tracks share them, so waiting minutes is normal). Use the PowerShell tool with timeout 600000; if the call times out, run the same command again (cargo resumes). PowerShell drops a bare -- when calling a script, so write it quoted: '--'. Never call cargo directly, never pass --workspace, --release or --profile, never run two cargo commands at once. While implementing use \`check -p <crate> --locked\` and \`test -p <crate> --locked\` on only the crates you changed, and \`check -p launcher --locked\` before each commit that touches game code.
RULES: no code comments of any kind (no //, /* */, ///, //!) in code you write; never run rustfmt or cargo fmt on existing files, edit only the lines you need in the surrounding style; a NEW .rs file must pass rustfmt on its staged content: the output of \`git show :<path> | rustfmt --check --edition <ed>\` must be EMPTY (it exits 0 even with a diff; edition 2021 for minecraft_terrain and the minecraftoss crates, 2024 otherwise); never commit Mojang files or data (read vanilla data at runtime; javap for checks: C:\\Program Files\\Eclipse Adoptium\\jdk-21.0.8.9-hotspot\\bin\\javap.exe -c -p -cp C:\\Games\\Mashup\\2010-Rust-Rewrite-Mashup\\iw4l-artifacts\\minecraft-26.3\\client.jar <class>); unit tests for logic; one commit per fix or feature with a plain-English subject (e.g. "Minecraft campfires cook food like vanilla") and the trailer line "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"; stage only your files. Keep all game state server-owned, per player and keyed by UUID (multiplayer comes next). Pick the most vanilla-like option when unsure and record the decision in the track log.
${A.avoid ? `FILES TO AVOID: ${A.avoid}` : ''}
TRACK LOG: ${LOG}. With every commit, update it in that same commit: subject, what changed, tests added, the owner's numbered in-game test steps and the kit items needed (minecraft: ids, counts), and any decision. A commit cannot hold its own hash, so name the previous commit's hash where useful. No amending.
BRIEFS: ${A.briefs}. Read them and the track log first; they were written earlier, so re-check line numbers.${A.notes ? `\nNOTES: ${A.notes}` : ''}`

phase('Build')
const done = []
for (const g of A.groups) {
  const prev = done.length ? `\n\nEarlier groups of this run already landed (see git log and the track log):\n${done.map(d => `- ${d.key}: ${String(d.summary).slice(0, 1500)}`).join('\n')}` : ''
  const res = await agent(`${RULES}${prev}\n\nYOUR GROUP: ${g.key}\n${g.prompt}\n\nFirst run \`git -C ${A.worktree} status\` and \`git log --oneline -15\`: if earlier work of this group is already committed (or a WIP commit exists), continue from it instead of redoing it. When the group is done (all its commits made, tests of touched crates green, check -p launcher green), return: the commits (hash + subject), tests added, anything not finished and why, and decisions taken.`, { label: `${A.track}:${g.key}`, phase: 'Build' })
  done.push({ key: g.key, summary: res })
}

let review = 'skipped'
if (!A.skipReview) {
  phase('Review')
  review = await agent(`${RULES}\n\nYou are the adversarial reviewer of this whole track. Base: ${A.base}. Run \`git log --oneline ${A.base}..HEAD\` and \`git diff ${A.base}..HEAD\` in the worktree (ignore merge commits that only bring in minecraft-survival). Group reports of this run:\n${done.map(d => `--- ${d.key} ---\n${String(d.summary).slice(0, 3000)}`).join('\n')}\n\nLook for real defects: logic bugs, vanilla mismatches (verify with javap where it matters), regressions on MW2 maps or in MW2 behaviour, state that is global where it must be per player, unfinished WIP commits, missing or meaningless tests, rule violations (code comments, reformatted existing lines, rustfmt output on new files, Mojang data committed). Fix every real defect in new commits (one per fix, plain-English subject, track log updated). Do not refactor for taste. Finish everything before you return: wait for every cargo command to end and commit every fix; your final message is the report itself, never a progress note like "waiting for the build". Return the defects found and the fix commits.`, { label: `${A.track}:review`, phase: 'Review' })
}

phase('Verify')
const verify = await agent(`${RULES}\n\nFinal verification of the track. 1) Find the crates the track touched: \`git diff --stat ${A.base}..HEAD\`. 2) Clippy on them: \`& .\\scripts\\cargo-slot.ps1 +target=${A.target} clippy -p <crate> -p <crate> --locked --all-targets '--' -A clippy::approx_constant\`; fix warnings on lines the track added (new commits); older warnings stay. 3) \`test -p <crate> --locked\` for each touched crate (do NOT run full-suite.ps1 or --workspace; the orchestrator runs the full suite at merge time). Known flaky: poof_particles::a_puff_rises_and_fades_within_its_lifetime and sections::tests::background_full_update_matches_the_synchronous_one (re-run once if only these fail). Fix real failures (new commits). 4) Make sure the track log has the owner's in-game test steps of every commit and one combined kit line (minecraft: ids, at most 27 stacks). 5) Finish everything before you return (never return a progress note). Return: clippy result, test results per crate, fixes made, and the track's commits since ${A.base}.`, { label: `${A.track}:verify`, phase: 'Verify' })

return { groups: done, review, verify }
