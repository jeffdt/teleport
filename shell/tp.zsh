tp() {
  local result=$(tp-core "$@")
  local rc=$?

  case "$result" in
    cd+c:*) cd "${result#cd+c:}" && claude ;;
    cd:*)   cd "${result#cd:}" ;;
    edit:*) ${=EDITOR:-vim} "${result#edit:}" ;;
    *)      [[ -n "$result" ]] && echo "$result" ;;
  esac

  return $rc
}

_tp() {
  local -a names
  if [[ -f ~/.config/tp/portals.toml ]]; then
    names=($(tp-core -l 2>/dev/null | awk '{print $1}'))
  fi
  _describe 'portal' names
}
compdef _tp tp
