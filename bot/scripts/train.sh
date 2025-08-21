#!/usr/bin/env bash

ROOT="$(cd "$(dirname "$0")" && pwd)/../.."
LOGS_DIR="$ROOT/bot/models/logs"

CFG_SIZE=17
BINBOT=burn_train
# BINBOT=train_ppo_burn
# BINBOT=train_dqn_burn
# BINBOT=train_dqn_burn_big
# BINBOT=train_dqn_burn_before
OPPONENT="random"

PLOT_EXT="png"

train() {
  ALGO=$1
  cargo build --release --bin=$BINBOT
  NAME="$(date +%Y-%m-%d_%H:%M:%S)"
  LOGS="$LOGS_DIR/$ALGO/$NAME.out"
  mkdir -p "$LOGS_DIR/$ALGO"
  LD_LIBRARY_PATH="$ROOT/target/release" "$ROOT/target/release/$BINBOT" $ALGO | tee "$LOGS"
}

plot() {
  ALGO=$1
  NAME=$(ls -rt "$LOGS_DIR/$ALGO" | tail -n 1)
  LOGS="$LOGS_DIR/$ALGO/$NAME"
  cfgs=$(head -n $CFG_SIZE "$LOGS")
  for cfg in $cfgs; do
    eval "$cfg"
  done

  # tail -n +$((CFG_SIZE + 2)) "$LOGS"
  tail -n +$((CFG_SIZE + 2)) "$LOGS" |
    grep -v "info:" |
    awk -F '[ ,]' '{print $5}' |
    feedgnuplot --lines --points --unset grid --title "adv = $OPPONENT ; density = $dense_size ; decay = $eps_decay ; max steps = $max_steps" --terminal $PLOT_EXT >"$LOGS_DIR/$ALGO/$NAME.$PLOT_EXT"
}

if [[ -z "$1" ]]; then
  echo "Usage : train [plot] <algo>"
elif [ "$1" = "plot" ]; then
  if [[ -z "$2" ]]; then
    echo "Usage : train [plot] <algo>"
  else
    plot $2
  fi
else
  train $1
fi
