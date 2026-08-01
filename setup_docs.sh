#!/bin/bash
mkdir -p docs/architecture/ADR docs/governance docs/development docs/planning docs/knowledge .claude/rules .clinerules

function move_or_create() {
  src=$1
  dest=$2
  if [ -f "$src" ]; then
    mv "$src" "$dest"
    echo "Moved $src to $dest"
  else
    if [ ! -f "$dest" ]; then
      touch "$dest"
      echo "Created $dest"
    fi
  fi
}

move_or_create "ARCHITECTURE.md" "docs/architecture/ARCHITECTURE.md"
move_or_create "DESIGN.md" "docs/architecture/DESIGN.md"
move_or_create "AUTH.md" "docs/governance/AUTH.md"
move_or_create "SECURITY.md" "docs/governance/SECURITY.md"
move_or_create "REVIEW.md" "docs/governance/REVIEW.md"
move_or_create "CONTRIBUTING.md" "docs/development/CONTRIBUTING.md"
move_or_create "TESTING.md" "docs/development/TESTING.md"
move_or_create "ONBOARDING.md" "docs/development/ONBOARDING.md"
move_or_create "spec.md" "docs/planning/spec.md"
move_or_create "plan.md" "docs/planning/plan.md"
move_or_create "tasks.md" "docs/planning/tasks.md"
move_or_create "MEMORY.md" "docs/knowledge/MEMORY.md"
move_or_create "CHANGELOG.md" "docs/knowledge/CHANGELOG.md"
move_or_create "GLOSSARY.md" "docs/knowledge/GLOSSARY.md"

touch AGENTS.md CLAUDE.md GEMINI.md SKILL.md
