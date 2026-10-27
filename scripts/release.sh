#!/bin/bash
# Release the current master version as-is (the workflow creates the tag).
# For the bump + tag + push flow, use scripts/git-release.sh.
gh workflow run release.yaml --ref master
