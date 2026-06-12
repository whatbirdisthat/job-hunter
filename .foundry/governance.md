# FOUNDRY merge governance — job-hunter

**Merge mode:** pr-approval

FOUNDRY builds the slice; the always-on adversarial review gate runs at every transition.
On PASS, FOUNDRY pushes the branch `slice-1-jd-to-tailored-cv` and opens a PR for the human
to merge. (This is a real public repo with a PII firewall — PR approval is the safe default.)
Switch any time: "give FOUNDRY merge autonomy" → direct-merge.

The adversarial review gate is always-on in BOTH modes; the mode only decides who merges after a PASS.
