#!/usr/bin/env sh

ROOT="$(cd "$(dirname "$0")" && pwd)/../.."
LOGS_DIR="$ROOT/bot/models/logs"

CFG_SIZE=12
# BINBOT=train_dqn_burn
BINBOT=train_dqn_burn_big
# BINBOT=train_dqn_burn_before
OPPONENT="random"

PLOT_EXT="png"

train() {
  cargo build --release --bin=$BINBOT
  NAME=$BINBOT"_$(date +%Y-%m-%d_%H:%M:%S)"
  LOGS="$LOGS_DIR/$NAME.out"
  mkdir -p "$LOGS_DIR"
  LD_LIBRARY_PATH="$ROOT/target/release" "$ROOT/target/release/$BINBOT" | tee "$LOGS"
}

plot() {
  NAME=$(ls -rt "$LOGS_DIR" | grep $BINBOT | tail -n 1)
  LOGS="$LOGS_DIR/$NAME"
  cfgs=$(head -n $CFG_SIZE "$LOGS")
  for cfg in $cfgs; do
    eval "$cfg"
  done

  # tail -n +$((CFG_SIZE + 2)) "$LOGS"
  tail -n +$((CFG_SIZE + 2)) "$LOGS" |
    grep -v "info:" |
    awk -F '[ ,]' '{print $5}' |
    feedgnuplot --lines --points --unset grid --title "adv = $OPPONENT ; density = $dense_size ; decay = $eps_decay ; max steps = $max_steps" --terminal $PLOT_EXT >"$LOGS_DIR/$OPPONENT-$dense_size-$eps_decay-$max_steps-$NAME.$PLOT_EXT"
}

if [ "$1" = "plot" ]; then
  plot
else
  train
fi
