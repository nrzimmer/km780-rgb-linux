#!/usr/bin/env bash
# Extracts KM780 protocol packets from a USBPcap capture.
#
#   ./extract.sh capture.pcapng [device_address]
#
# Output, one line per packet:  <frame> <t_rel> <DIR> <cmd> <payload hex>
#   SET  host -> keyboard, 264 bytes (Feature report ID 7, SET_REPORT)
#   GET  keyboard's answer to a control IN, 264 bytes
#   ACK  8 bytes on endpoint 0x83
#
# The payload is always the LAST bytes of the frame: USBPcap puts 28 bytes of
# pseudo-header (+8 of setup on SET) in front.

# On Windows (git-bash) tshark is not on PATH; on Linux it is.
TSHARK="${TSHARK:-}"
[ -n "$TSHARK" ] || TSHARK=$(command -v tshark 2>/dev/null)
[ -n "$TSHARK" ] || TSHARK="/c/Program Files/Wireshark/tshark.exe"
CAP="$1"
ADDR="${2:-2}"
FILTER="usb.device_address==$ADDR && ((usb.transfer_type==0x02 && usb.data_len>=264) || (usb.endpoint_address==0x83 && usb.data_len==8 && usb.src!=\"host\"))"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

"$TSHARK" -r "$CAP" -Y "$FILTER" -T fields \
  -e frame.number -e frame.time_relative -e usb.bmRequestType -e usb.endpoint_address \
  2>/dev/null > "$TMP/meta"

"$TSHARK" -r "$CAP" -Y "$FILTER" -T json -x 2>/dev/null \
  | grep -A1 '"frame_raw"' | grep -o '"[0-9a-f]\{16,\}"' | tr -d '"' > "$TMP/hex"

[ "$(wc -l < "$TMP/meta")" = "$(wc -l < "$TMP/hex")" ] || {
  echo "ERROR: meta=$(wc -l < "$TMP/meta") hex=$(wc -l < "$TMP/hex") -- misaligned" >&2
  exit 1
}

paste "$TMP/meta" "$TMP/hex" | awk -F'\t' '
{
  gsub(/[\r]/, "")
  frame=$1; t=$2; brt=$3; ep=$4; hex=$5; n=length(hex)
  if (ep == "0x83")     { dir="ACK"; pay=substr(hex, n-15);  cmd=substr(pay,5,2) }
  else if (brt=="0x21") { dir="SET"; pay=substr(hex, n-527); cmd=substr(pay,3,2) }
  else                  { dir="GET"; pay=substr(hex, n-527); cmd=substr(pay,3,2) }
  printf "%s\t%.6f\t%s\t0x%s\t%s\n", frame, t, dir, cmd, pay
}'
