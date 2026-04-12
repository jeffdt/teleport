tp() {
  if [[ "$1" == "edit" ]]; then
    ${EDITOR:-vim} ~/.config/tp/portals.toml
    return
  fi

  local claude=false
  local args=()
  for arg in "$@"; do
    if [[ "$arg" == "-c" || "$arg" == "--claude" ]]; then
      claude=true
    else
      args+=("$arg")
    fi
  done

  local result=$(warp-core "${args[@]}")
  local rc=$?

  case "$result" in
    cd:*)
      cd "${result#cd:}"
      $claude && claude
      ;;
    *) [[ -n "$result" ]] && echo "$result" ;;
  esac

  return $rc
}

_tp() {
  local -a names
  if [[ -f ~/.config/tp/portals.toml ]]; then
    names=($(warp-core ls 2>/dev/null | awk '{print $1}'))
  fi
  _describe 'portal' names
}
compdef _tp tp
