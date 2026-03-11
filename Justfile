set shell := ["bash", "-cu"]

default:
  @just --list

demo-single:
  v=50; while true; do step=$((RANDOM % 11 - 5)); v=$((v + step)); (( v < 0 )) && v=0; printf '%s\n' "$v"; sleep 0.08; done | cargo run --release -- --update-frequency 80 --frame-rate 30

demo-multi:
  a=40; b=70; c=20; while true; do a=$((a + RANDOM % 9 - 4)); b=$((b + RANDOM % 13 - 6)); c=$((c + RANDOM % 7 - 3)); (( a < 0 )) && a=0; (( b < 0 )) && b=0; (( c < 0 )) && c=0; printf '%s %s %s\n' "$a" "$b" "$c"; sleep 0.08; done | cargo run --release -- --update-frequency 80 --frame-rate 30

demo-group:
  a=30; b=55; c=80; while true; do a=$((a + RANDOM % 11 - 5)); b=$((b + RANDOM % 11 - 5)); c=$((c + RANDOM % 11 - 5)); (( a < 0 )) && a=0; (( b < 0 )) && b=0; (( c < 0 )) && c=0; printf '%s %s %s\n' "$a" "$b" "$c"; sleep 0.08; done | cargo run --release -- -g --update-frequency 80 --frame-rate 30

demo-spike:
  i=0; base=40; while true; do i=$((i + 1)); v=$((base + RANDOM % 7 - 3)); if (( i % 20 == 0 )); then v=$((v + 40)); fi; printf '%s\n' "$v"; sleep 0.08; done | cargo run --release -- --update-frequency 80 --frame-rate 30

demo-sine:
  values=(40 44 48 52 56 59 62 64 66 67 68 67 66 64 62 59 56 52 48 44 40 36 32 28 24 21 18 16 14 13 12 13 14 16 18 21 24 28 32 36); i=0; n=${#values[@]}; while true; do printf '%s\n' "${values[i]}"; i=$(((i + 1) % n)); sleep 0.08; done | cargo run --release -- --update-frequency 80 --frame-rate 30

demo-sine-smooth:
  values=(40 42 44 46 48 50 52 54 56 58 60 61 62 63 64 65 66 67 68 67 66 65 64 63 62 61 60 58 56 54 52 50 48 46 44 42 40 38 36 34 32 30 28 26 24 22 20 19 18 17 16 15 14 13 12 13 14 15 16 17 18 19 20 22 24 26 28 30 32 34 36 38); i=0; n=${#values[@]}; while true; do printf '%s\n' "${values[i]}"; i=$(((i + 1) % n)); sleep 0.033; done | cargo run --release -- --update-frequency 33 --frame-rate 60
