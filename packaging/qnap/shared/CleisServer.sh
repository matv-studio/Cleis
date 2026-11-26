#!/bin/sh
# Starts and stops Cleis Server on a QNAP NAS (D-78). QTS runs it at boot, and from App Center.
# The organization's data lives on the default volume, outside the package folder, so removing
# or upgrading the package keeps it: <volume>/.cleis-server (server.db, cleis-server.toml,
# backups). Created by the Cleis app: `cleis-server init --data <that folder> --org NAME`.
CONF=/etc/config/qpkg.conf
QPKG_NAME="CleisServer"
QPKG_ROOT=$(/sbin/getcfg $QPKG_NAME Install_Path -f ${CONF})
VOLUME=$(/sbin/getcfg SHARE_DEF defVolMP -f /etc/config/def_share.info)
DATA="${CLEIS_DATA:-$VOLUME/.cleis-server}"
PID=/var/run/cleis-server.pid
LOG="$DATA/cleis-server.log"
export QNAP_QPKG=$QPKG_NAME

running() {
    [ -f "$PID" ] && kill -0 "$(cat "$PID")" 2>/dev/null
}

case "$1" in
  start)
    ENABLED=$(/sbin/getcfg $QPKG_NAME Enable -u -d FALSE -f $CONF)
    if [ "$ENABLED" != "TRUE" ]; then
        echo "$QPKG_NAME is disabled."
        exit 1
    fi
    if [ ! -f "$DATA/server.db" ]; then
        echo "No organization yet in $DATA: create it from the Cleis app."
        exit 0
    fi
    running && exit 0
    cd "$DATA" || exit 1
    "$QPKG_ROOT/cleis-server" run --data "$DATA" >>"$LOG" 2>&1 &
    echo $! > "$PID"
    ;;
  stop)
    if running; then
        kill "$(cat "$PID")"
        for _ in 1 2 3 4 5 6 7 8 9 10; do running || break; sleep 1; done
        running && kill -9 "$(cat "$PID")"
    fi
    rm -f "$PID"
    ;;
  restart)
    $0 stop
    $0 start
    ;;
  status)
    if running; then echo "running"; else echo "stopped"; exit 1; fi
    ;;
  data)
    # Where the organization lives (read by the Cleis app over SSH).
    echo "$DATA"
    ;;
  remove)
    ;;
  *)
    echo "Usage: $0 {start|stop|restart|status|data}"
    exit 1
esac
exit 0
