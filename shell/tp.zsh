tp() {
  if [[ "$1" == "edit" ]]; then
    ${EDITOR:-vim} ~/.config/tp/portals.toml
    return
  fi

  local claude=false
  local args=("$@")
  if [[ "$1" == "-c" ]]; then
    claude=true
    args=("${@:2}")
  fi

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
  _describe 'portal/tunnel' names
}
compdef _tp tp
