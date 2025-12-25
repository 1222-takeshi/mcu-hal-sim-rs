#!/usr/bin/env bash

set -euo pipefail

SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"

show_help() {
  cat <<EOF
Usage: ${SCRIPT_NAME} <command> [options]

Commands:
  push    Push current or specified branch
  pr      Create a GitHub pull request
  issue   Create a GitHub issue

Run '${SCRIPT_NAME} <command> --help' for command-specific options.
EOF
}

ensure_git_repo() {
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "Error: this script must be run inside a Git repository." >&2
    exit 1
  fi
}

get_current_branch() {
  git rev-parse --abbrev-ref HEAD
}

subcommand_push() {
  ensure_git_repo

  local branch=""
  local remote="origin"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -b|--branch)
        branch="${2:-}"
        shift 2
        ;;
      -r|--remote)
        remote="${2:-}"
        shift 2
        ;;
      -h|--help)
        cat <<EOF
Usage: ${SCRIPT_NAME} push [-b BRANCH] [-r REMOTE]

Options:
  -b, --branch BRANCH   Branch to push (default: current branch)
  -r, --remote REMOTE   Remote name (default: origin)
EOF
        return 0
        ;;
      *)
        echo "Error: unknown option for 'push': $1" >&2
        return 1
        ;;
    esac
  done

  if [[ -z "$branch" ]]; then
    branch="$(get_current_branch)"
  fi

  echo "Pushing branch '${branch}' to remote '${remote}'..."
  git push "$remote" "$branch"
}

subcommand_pr() {
  ensure_git_repo

  local branch=""
  local base=""
  local title=""
  local body=""
  local body_file=""
  local draft="false"
  local fill="false"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -b|--branch)
        branch="${2:-}"
        shift 2
        ;;
      -B|--base)
        base="${2:-}"
        shift 2
        ;;
      -t|--title)
        title="${2:-}"
        shift 2
        ;;
      --body)
        body="${2:-}"
        shift 2
        ;;
      --body-file)
        body_file="${2:-}"
        shift 2
        ;;
      --draft)
        draft="true"
        shift
        ;;
      --fill)
        fill="true"
        shift
        ;;
      -h|--help)
        cat <<EOF
Usage: ${SCRIPT_NAME} pr [options]

Options:
  -b, --branch BRANCH     Source branch (default: current branch)
  -B, --base BRANCH       Base branch (e.g. main)
  -t, --title TITLE       Pull request title
      --body TEXT         Pull request body text
      --body-file PATH    Pull request body from file
      --draft             Create as draft pull request
      --fill              Use commit messages / template for title and body

Notes:
  - If neither --title/--body/--body-file is specified, --fill is used by default.
EOF
        return 0
        ;;
      *)
        echo "Error: unknown option for 'pr': $1" >&2
        return 1
        ;;
    esac
  done

  if [[ -z "$branch" ]]; then
    branch="$(get_current_branch)"
  fi

  local args=()
  args+=("--head" "$branch")

  if [[ -n "$base" ]]; then
    args+=("--base" "$base")
  fi

  if [[ -n "$title" ]]; then
    args+=("--title" "$title")
  fi

  if [[ -n "$body_file" ]]; then
    args+=("--body-file" "$body_file")
  elif [[ -n "$body" ]]; then
    args+=("--body" "$body")
  fi

  if [[ "$draft" == "true" ]]; then
    args+=("--draft")
  fi

  if [[ "$fill" == "true" ]] || { [[ -z "$title" && -z "$body" && -z "$body_file" ]]; }; then
    args+=("--fill")
  fi

  echo "Creating pull request from branch '${branch}'..."
  gh pr create "${args[@]}"
}

subcommand_issue() {
  ensure_git_repo

  local title=""
  local body=""
  local body_file=""
  local assignee=""
  local label=""
  local milestone=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -t|--title)
        title="${2:-}"
        shift 2
        ;;
      --body)
        body="${2:-}"
        shift 2
        ;;
      --body-file)
        body_file="${2:-}"
        shift 2
        ;;
      -a|--assignee)
        assignee="${2:-}"
        shift 2
        ;;
      -l|--label)
        label="${2:-}"
        shift 2
        ;;
      -m|--milestone)
        milestone="${2:-}"
        shift 2
        ;;
      -h|--help)
        cat <<EOF
Usage: ${SCRIPT_NAME} issue -t TITLE [options]

Options:
  -t, --title TITLE       Issue title (required)
      --body TEXT         Issue body text
      --body-file PATH    Issue body from file
  -a, --assignee USER     Assignee
  -l, --label LABEL       Label
  -m, --milestone NAME    Milestone
EOF
        return 0
        ;;
      *)
        echo "Error: unknown option for 'issue': $1" >&2
        return 1
        ;;
    esac
  done

  if [[ -z "$title" ]]; then
    echo "Error: --title is required for 'issue' command." >&2
    return 1
  fi

  local args=("--title" "$title")

  if [[ -n "$body_file" ]]; then
    args+=("--body-file" "$body_file")
  elif [[ -n "$body" ]]; then
    args+=("--body" "$body")
  fi

  if [[ -n "$assignee" ]]; then
    args+=("--assignee" "$assignee")
  fi

  if [[ -n "$label" ]]; then
    args+=("--label" "$label")
  fi

  if [[ -n "$milestone" ]]; then
    args+=("--milestone" "$milestone")
  fi

  echo "Creating issue '${title}'..."
  gh issue create "${args[@]}"
}

main() {
  if [[ $# -lt 1 ]]; then
    show_help
    exit 1
  fi

  local cmd="$1"
  shift || true

  case "$cmd" in
    push)
      subcommand_push "$@"
      ;;
    pr)
      subcommand_pr "$@"
      ;;
    issue)
      subcommand_issue "$@"
      ;;
    -h|--help|help)
      show_help
      ;;
    *)
      echo "Error: unknown command '${cmd}'" >&2
      echo
      show_help
      exit 1
      ;;
  esac
}

main "$@"

