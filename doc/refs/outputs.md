# Outputs

## 50 episodes - 1000 steps max - desktop

{"episode": 0, "reward": -1798.7162, "steps count": 1000, "duration": 11}
{"episode": 1, "reward": -1794.8162, "steps count": 1000, "duration": 32}
{"episode": 2, "reward": -1387.7109, "steps count": 1000, "duration": 58}
{"episode": 3, "reward": -42.5005, "steps count": 1000, "duration": 82}
{"episode": 4, "reward": -48.2005, "steps count": 1000, "duration": 109}
{"episode": 5, "reward": 1.2000, "steps count": 1000, "duration": 141}
{"episode": 6, "reward": 8.8000, "steps count": 1000, "duration": 184}
{"episode": 7, "reward": 6.9002, "steps count": 1000, "duration": 219}
{"episode": 8, "reward": 16.5001, "steps count": 1000, "duration": 248}
{"episode": 9, "reward": -2.6000, "steps count": 1000, "duration": 281}
{"episode": 10, "reward": 3.0999, "steps count": 1000, "duration": 324}
{"episode": 11, "reward": -34.7004, "steps count": 1000, "duration": 497}
{"episode": 12, "reward": -15.7998, "steps count": 1000, "duration": 466}
{"episode": 13, "reward": 6.9000, "steps count": 1000, "duration": 496}
{"episode": 14, "reward": 6.3000, "steps count": 1000, "duration": 540}
{"episode": 15, "reward": -2.6000, "steps count": 1000, "duration": 581}
{"episode": 16, "reward": -33.0003, "steps count": 1000, "duration": 641}
{"episode": 17, "reward": -36.8000, "steps count": 1000, "duration": 665}
{"episode": 18, "reward": -10.1997, "steps count": 1000, "duration": 753}
{"episode": 19, "reward": -88.1014, "steps count": 1000, "duration": 837}
{"episode": 20, "reward": -57.5002, "steps count": 1000, "duration": 881}
{"episode": 21, "reward": -17.7997, "steps count": 1000, "duration": 1159}
{"episode": 22, "reward": -25.4000, "steps count": 1000, "duration": 1235}
{"episode": 23, "reward": -104.4013, "steps count": 995, "duration": 1290}
{"episode": 24, "reward": -268.6004, "steps count": 1000, "duration": 1322}
{"episode": 25, "reward": -743.6052, "steps count": 1000, "duration": 1398}
{"episode": 26, "reward": -821.5029, "steps count": 1000, "duration": 1427}
{"episode": 27, "reward": -211.5993, "steps count": 1000, "duration": 1409}
{"episode": 28, "reward": -276.1974, "steps count": 1000, "duration": 1463}
{"episode": 29, "reward": -222.9980, "steps count": 1000, "duration": 1509}
{"episode": 30, "reward": -298.9973, "steps count": 1000, "duration": 1560}
{"episode": 31, "reward": -164.0011, "steps count": 1000, "duration": 1752}
{"episode": 32, "reward": -221.0990, "steps count": 1000, "duration": 1807}
{"episode": 33, "reward": -260.9996, "steps count": 1000, "duration": 1730}
{"episode": 34, "reward": -420.5959, "steps count": 1000, "duration": 1767}
{"episode": 35, "reward": -407.2964, "steps count": 1000, "duration": 1815}
{"episode": 36, "reward": -291.2966, "steps count": 1000, "duration": 1870}

thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
error: Recipe `trainbot` was terminated on line 24 by signal 6

## 50 episodes - 700 steps max - desktop

const MEMORY_SIZE: usize = 4096;
const DENSE_SIZE: usize = 128;
const EPS_DECAY: f64 = 1000.0;
const EPS_START: f64 = 0.9;
const EPS_END: f64 = 0.05;

> Entraînement
> {"episode": 0, "reward": -862.8993, "steps count": 700, "duration": 6}
> {"episode": 1, "reward": -418.8971, "steps count": 700, "duration": 13}
> {"episode": 2, "reward": -64.9999, "steps count": 453, "duration": 14}
> {"episode": 3, "reward": -142.8002, "steps count": 700, "duration": 31}
> {"episode": 4, "reward": -74.4004, "steps count": 700, "duration": 45}
> {"episode": 5, "reward": -40.2002, "steps count": 700, "duration": 58}
> {"episode": 6, "reward": -21.1998, "steps count": 700, "duration": 70}
> {"episode": 7, "reward": 99.7000, "steps count": 642, "duration": 79}
> {"episode": 8, "reward": -5.9999, "steps count": 700, "duration": 99}
> {"episode": 9, "reward": -7.8999, "steps count": 700, "duration": 118}
> {"episode": 10, "reward": 92.5000, "steps count": 624, "duration": 117}
> {"episode": 11, "reward": -17.1998, "steps count": 700, "duration": 144}
> {"episode": 12, "reward": 1.7000, "steps count": 700, "duration": 157}
> {"episode": 13, "reward": -7.9000, "steps count": 700, "duration": 172}
> {"episode": 14, "reward": -7.9000, "steps count": 700, "duration": 196}
> {"episode": 15, "reward": -2.8000, "steps count": 700, "duration": 214}
> {"episode": 16, "reward": 16.8002, "steps count": 700, "duration": 250}
> {"episode": 17, "reward": -47.7001, "steps count": 700, "duration": 272}
> k{"episode": 18, "reward": -13.6000, "steps count": 700, "duration": 288}
> {"episode": 19, "reward": -79.9002, "steps count": 700, "duration": 304}
> {"episode": 20, "reward": -355.5985, "steps count": 700, "duration": 317}
> {"episode": 21, "reward": -205.5001, "steps count": 700, "duration": 333}
> {"episode": 22, "reward": -207.3974, "steps count": 700, "duration": 348}
> {"episode": 23, "reward": -161.7999, "steps count": 700, "duration": 367}

---

const MEMORY_SIZE: usize = 8192;
const DENSE_SIZE: usize = 128;
const EPS_DECAY: f64 = 10000.0;
const EPS_START: f64 = 0.9;
const EPS_END: f64 = 0.05;

> Entraînement
> {"episode": 0, "reward": -1119.9921, "steps count": 700, "duration": 6}
> {"episode": 1, "reward": -928.6963, "steps count": 700, "duration": 13}
> {"episode": 2, "reward": -364.5009, "steps count": 380, "duration": 11}
> {"episode": 3, "reward": -797.5981, "steps count": 700, "duration": 28}
> {"episode": 4, "reward": -577.5994, "steps count": 599, "duration": 34}
> {"episode": 5, "reward": -725.2992, "steps count": 700, "duration": 49}
> {"episode": 6, "reward": -638.8995, "steps count": 700, "duration": 59}
> {"episode": 7, "reward": -1039.1932, "steps count": 700, "duration": 73}
> field invalid : White, 3, Board { positions: [13, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, -1, -2, 0, -11] }

thread 'main' panicked at store/src/game.rs:556:65:
called `Result::unwrap()` on an `Err` value: FieldInvalid
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: Recipe `trainbot` failed on line 27 with exit code 101

---

# [allow(unused)]

const MEMORY_SIZE: usize = 8192;
const DENSE_SIZE: usize = 256;
const EPS_DECAY: f64 = 10000.0;
const EPS_START: f64 = 0.9;
const EPS_END: f64 = 0.05;

> Entraînement
> {"episode": 0, "reward": -1102.6925, "steps count": 700, "duration": 9}
> field invalid : White, 6, Board { positions: [14, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, -1, 0, 0, -13] }

thread 'main' panicked at store/src/game.rs:556:65:
called `Result::unwrap()` on an `Err` value: FieldInvalid
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: Recipe `trainbot` failed on line 27 with exit code 101

---

const MEMORY_SIZE: usize = 8192;
const DENSE_SIZE: usize = 256;
const EPS_DECAY: f64 = 1000.0;
const EPS_START: f64 = 0.9;
const EPS_END: f64 = 0.05;

> Entraînement
> {"episode": 0, "reward": -1116.2921, "steps count": 700, "duration": 9}
> {"episode": 1, "reward": -1116.2922, "steps count": 700, "duration": 18}
> {"episode": 2, "reward": -1119.9921, "steps count": 700, "duration": 29}
> {"episode": 3, "reward": -1089.1927, "steps count": 700, "duration": 41}
> {"episode": 4, "reward": -1116.2921, "steps count": 700, "duration": 53}
> {"episode": 5, "reward": -684.8043, "steps count": 700, "duration": 66}
> {"episode": 6, "reward": 0.3000, "steps count": 700, "duration": 80}
> {"episode": 7, "reward": 2.0000, "steps count": 700, "duration": 96}
> {"episode": 8, "reward": 30.9001, "steps count": 700, "duration": 112}
> {"episode": 9, "reward": 0.3000, "steps count": 700, "duration": 128}
> {"episode": 10, "reward": 0.3000, "steps count": 700, "duration": 141}
> {"episode": 11, "reward": 8.8000, "steps count": 700, "duration": 155}
> {"episode": 12, "reward": 7.1000, "steps count": 700, "duration": 169}
> {"episode": 13, "reward": 17.3001, "steps count": 700, "duration": 190}
> {"episode": 14, "reward": -107.9005, "steps count": 700, "duration": 210}
> {"episode": 15, "reward": 7.1001, "steps count": 700, "duration": 236}
> {"episode": 16, "reward": 17.3001, "steps count": 700, "duration": 268}
> {"episode": 17, "reward": 7.1000, "steps count": 700, "duration": 283}
> {"episode": 18, "reward": -5.9000, "steps count": 700, "duration": 300}
> {"episode": 19, "reward": -36.8009, "steps count": 700, "duration": 316}
> {"episode": 20, "reward": 19.0001, "steps count": 700, "duration": 332}
> {"episode": 21, "reward": 113.3000, "steps count": 461, "duration": 227}
> field invalid : White, 1, Board { positions: [0, 2, 2, 0, 2, 4, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -3, -7, -2, -1, 0, -1, -1] }

thread 'main' panicked at store/src/game.rs:556:65:
called `Result::unwrap()` on an `Err` value: FieldInvalid
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: Recipe `trainbot` failed on line 27 with exit code 101

---

num_episodes: 50,
// memory_size: 8192, // must be set in dqn_model.rs with the MEMORY_SIZE constant
// max_steps: 700, // must be set in environment.rs with the MAX_STEPS constant
dense_size: 256, // neural network complexity
eps_start: 0.9, // epsilon initial value (0.9 => more exploration)
eps_end: 0.05,
eps_decay: 1000.0,

> Entraînement
> {"episode": 0, "reward": -1118.8921, "steps count": 700, "duration": 9}
> {"episode": 1, "reward": -1119.9921, "steps count": 700, "duration": 17}
> {"episode": 2, "reward": -1118.8921, "steps count": 700, "duration": 28}
> {"episode": 3, "reward": -283.5977, "steps count": 700, "duration": 41}
> {"episode": 4, "reward": -23.4998, "steps count": 700, "duration": 54}
> {"episode": 5, "reward": -31.9999, "steps count": 700, "duration": 68}
> {"episode": 6, "reward": 2.0000, "steps count": 700, "duration": 82}
> {"episode": 7, "reward": 109.3000, "steps count": 192, "duration": 26}
> {"episode": 8, "reward": -4.8000, "steps count": 700, "duration": 102}
> {"episode": 9, "reward": 15.6001, "steps count": 700, "duration": 124}
> {"episode": 10, "reward": 15.6002, "steps count": 700, "duration": 144}
> {"episode": 11, "reward": -65.7008, "steps count": 700, "duration": 162}
> {"episode": 12, "reward": 19.0002, "steps count": 700, "duration": 182}
> {"episode": 13, "reward": 20.7001, "steps count": 700, "duration": 197}
> {"episode": 14, "reward": 12.2002, "steps count": 700, "duration": 229}
> {"episode": 15, "reward": -32.0007, "steps count": 700, "duration": 242}
> {"episode": 16, "reward": 10.5000, "steps count": 700, "duration": 287}
> {"episode": 17, "reward": 24.1001, "steps count": 700, "duration": 318}
> {"episode": 18, "reward": 25.8002, "steps count": 700, "duration": 335}
> {"episode": 19, "reward": 29.2001, "steps count": 700, "duration": 367}
> {"episode": 20, "reward": 9.1000, "steps count": 700, "duration": 366}
> {"episode": 21, "reward": 3.7001, "steps count": 700, "duration": 398}
> {"episode": 22, "reward": 10.5000, "steps count": 700, "duration": 417}
> {"episode": 23, "reward": 10.5000, "steps count": 700, "duration": 438}
> {"episode": 24, "reward": 13.9000, "steps count": 700, "duration": 444}
> {"episode": 25, "reward": 7.1000, "steps count": 700, "duration": 486}
> {"episode": 26, "reward": 12.2001, "steps count": 700, "duration": 499}
> {"episode": 27, "reward": 8.8001, "steps count": 700, "duration": 554}
> {"episode": 28, "reward": -6.5000, "steps count": 700, "duration": 608}
> {"episode": 29, "reward": -3.1000, "steps count": 700, "duration": 633}
> {"episode": 30, "reward": -32.0001, "steps count": 700, "duration": 696}
> {"episode": 31, "reward": 22.4002, "steps count": 700, "duration": 843}
> {"episode": 32, "reward": -77.9004, "steps count": 700, "duration": 817}
> {"episode": 33, "reward": -368.5993, "steps count": 700, "duration": 827}
> {"episode": 34, "reward": -254.6986, "steps count": 700, "duration": 852}
> {"episode": 35, "reward": -433.1992, "steps count": 700, "duration": 884}
> {"episode": 36, "reward": -521.6010, "steps count": 700, "duration": 905}
> {"episode": 37, "reward": -71.1004, "steps count": 700, "duration": 930}
> {"episode": 38, "reward": -251.0004, "steps count": 700, "duration": 956}
> {"episode": 39, "reward": -594.7045, "steps count": 700, "duration": 982}
> {"episode": 40, "reward": -154.4001, "steps count": 700, "duration": 1008}
> {"episode": 41, "reward": -171.3994, "steps count": 700, "duration": 1033}
> {"episode": 42, "reward": -118.7004, "steps count": 700, "duration": 1059}
> {"episode": 43, "reward": -137.4003, "steps count": 700, "duration": 1087}

thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
error: Recipe `trainbot` was terminated on line 27 by signal 6

---

num_episodes: 40,
// memory_size: 8192, // must be set in dqn_model.rs with the MEMORY_SIZE constant
// max_steps: 1500, // must be set in environment.rs with the MAX_STEPS constant
dense_size: 256, // neural network complexity
eps_start: 0.9, // epsilon initial value (0.9 => more exploration)
eps_end: 0.05,
eps_decay: 1000.0,

> Entraînement
> {"episode": 0, "reward": -2399.9993, "steps count": 1500, "duration": 31}
> {"episode": 1, "reward": -2061.6736, "steps count": 1500, "duration": 81}
> {"episode": 2, "reward": -48.9010, "steps count": 1500, "duration": 145}
> {"episode": 3, "reward": 3.8000, "steps count": 1500, "duration": 215}
> {"episode": 4, "reward": -6.3999, "steps count": 1500, "duration": 302}
> {"episode": 5, "reward": 20.8004, "steps count": 1500, "duration": 374}
> {"episode": 6, "reward": 49.6992, "steps count": 1500, "duration": 469}
> {"episode": 7, "reward": 29.3002, "steps count": 1500, "duration": 597}
> {"episode": 8, "reward": 34.3999, "steps count": 1500, "duration": 710}
> {"episode": 9, "reward": 115.3003, "steps count": 966, "duration": 515}
> {"episode": 10, "reward": 25.9004, "steps count": 1500, "duration": 852}
> {"episode": 11, "reward": -122.0007, "steps count": 1500, "duration": 1017}
> {"episode": 12, "reward": -274.9966, "steps count": 1500, "duration": 1073}
> {"episode": 13, "reward": 54.8994, "steps count": 651, "duration": 518}
> {"episode": 14, "reward": -439.8978, "steps count": 1500, "duration": 1244}
> {"episode": 15, "reward": -506.1997, "steps count": 1500, "duration": 1676}
> {"episode": 16, "reward": -829.5031, "steps count": 1500, "duration": 1855}
> {"episode": 17, "reward": -545.2961, "steps count": 1500, "duration": 1892}
> {"episode": 18, "reward": -795.2026, "steps count": 1500, "duration": 2008}
> {"episode": 19, "reward": -637.1031, "steps count": 1500, "duration": 2124}
> {"episode": 20, "reward": -989.6997, "steps count": 1500, "duration": 2241}

thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
error: Recipe `trainbot` was terminated on line 27 by signal 6

---

num_episodes: 40,
// memory_size: 8192, // must be set in dqn_model.rs with the MEMORY_SIZE constant
// max_steps: 1000, // must be set in environment.rs with the MAX_STEPS constant
dense_size: 256, // neural network complexity
eps_start: 0.9, // epsilon initial value (0.9 => more exploration)
eps_end: 0.05,
eps_decay: 10000.0,

> Entraînement
> {"episode": 0, "reward": -1598.8848, "steps count": 1000, "duration": 16}
> {"episode": 1, "reward": -1531.9866, "steps count": 1000, "duration": 34}
> {"episode": 2, "reward": -515.6000, "steps count": 530, "duration": 25}
> {"episode": 3, "reward": -396.1008, "steps count": 441, "duration": 27}
> {"episode": 4, "reward": -540.6996, "steps count": 605, "duration": 43}
> {"episode": 5, "reward": -976.0975, "steps count": 1000, "duration": 89}
> {"episode": 6, "reward": -1014.2944, "steps count": 1000, "duration": 117}
> {"episode": 7, "reward": -806.7012, "steps count": 1000, "duration": 140}
> {"episode": 8, "reward": -1276.6891, "steps count": 1000, "duration": 166}
> {"episode": 9, "reward": -1554.3855, "steps count": 1000, "duration": 197}
> {"episode": 10, "reward": -1178.3925, "steps count": 1000, "duration": 219}
> {"episode": 11, "reward": -1457.4869, "steps count": 1000, "duration": 258}
> {"episode": 12, "reward": -1475.8882, "steps count": 1000, "duration": 291}

---

num_episodes: 40,
// memory_size: 8192, // must be set in dqn_model.rs with the MEMORY_SIZE constant
// max_steps: 1000, // must be set in environment.rs with the MAX_STEPS constant
dense_size: 256, // neural network complexity
eps_start: 0.9, // epsilon initial value (0.9 => more exploration)
eps_end: 0.05,
eps_decay: 3000.0,

> Entraînement
> {"episode": 0, "reward": -1598.8848, "steps count": 1000, "duration": 15}
> {"episode": 1, "reward": -1599.9847, "steps count": 1000, "duration": 33}
> {"episode": 2, "reward": -751.7018, "steps count": 1000, "duration": 57}
> {"episode": 3, "reward": -402.8979, "steps count": 1000, "duration": 81}
> {"episode": 4, "reward": -289.2985, "steps count": 1000, "duration": 108}
> {"episode": 5, "reward": -231.4988, "steps count": 1000, "duration": 140}
> {"episode": 6, "reward": -138.0006, "steps count": 1000, "duration": 165}
> {"episode": 7, "reward": -145.0998, "steps count": 1000, "duration": 200}
> {"episode": 8, "reward": -60.4005, "steps count": 1000, "duration": 236}
> {"episode": 9, "reward": -35.7999, "steps count": 1000, "duration": 276}
> {"episode": 10, "reward": -42.2002, "steps count": 1000, "duration": 313}
> {"episode": 11, "reward": 69.0002, "steps count": 874, "duration": 300}
> {"episode": 12, "reward": 93.2000, "steps count": 421, "duration": 153}
> {"episode": 13, "reward": -324.9010, "steps count": 866, "duration": 364}
> {"episode": 14, "reward": -1331.3883, "steps count": 1000, "duration": 478}
> {"episode": 15, "reward": -1544.5859, "steps count": 1000, "duration": 514}
> {"episode": 16, "reward": -1599.9847, "steps count": 1000, "duration": 552}

---

Nouveaux points...

num_episodes: 40,
// memory_size: 8192, // must be set in dqn_model.rs with the MEMORY_SIZE constant
// max_steps: 1000, // must be set in environment.rs with the MAX_STEPS constant
dense_size: 256, // neural network complexity
eps_start: 0.9, // epsilon initial value (0.9 => more exploration)
eps_end: 0.05,
eps_decay: 3000.0,

> Entraînement
> {"episode": 0, "reward": -1798.1161, "steps count": 1000, "duration": 15}
> {"episode": 1, "reward": -1800.0162, "steps count": 1000, "duration": 34}
> {"episode": 2, "reward": -1718.6151, "steps count": 1000, "duration": 57}
> {"episode": 3, "reward": -1369.5055, "steps count": 1000, "duration": 82}
> {"episode": 4, "reward": -321.5974, "steps count": 1000, "duration": 115}
> {"episode": 5, "reward": -213.2988, "steps count": 1000, "duration": 148}
> {"episode": 6, "reward": -175.4995, "steps count": 1000, "duration": 172}
> {"episode": 7, "reward": -126.1011, "steps count": 1000, "duration": 203}
> {"episode": 8, "reward": -105.1011, "steps count": 1000, "duration": 242}
> {"episode": 9, "reward": -46.3007, "steps count": 1000, "duration": 281}
> {"episode": 10, "reward": -57.7006, "steps count": 1000, "duration": 323}
> {"episode": 11, "reward": -15.7997, "steps count": 1000, "duration": 354}
> {"episode": 12, "reward": -38.6999, "steps count": 1000, "duration": 414}
> {"episode": 13, "reward": 10.7002, "steps count": 1000, "duration": 513}
> {"episode": 14, "reward": -10.1999, "steps count": 1000, "duration": 585}
> {"episode": 15, "reward": -8.3000, "steps count": 1000, "duration": 644}
> {"episode": 16, "reward": -463.4984, "steps count": 973, "duration": 588}
> {"episode": 17, "reward": -148.8951, "steps count": 1000, "duration": 646}
> {"episode": 18, "reward": 3.0999, "steps count": 1000, "duration": 676}
> {"episode": 19, "reward": -12.0999, "steps count": 1000, "duration": 753}
> {"episode": 20, "reward": 6.9000, "steps count": 1000, "duration": 801}
> {"episode": 21, "reward": 14.5001, "steps count": 1000, "duration": 850}
> {"episode": 22, "reward": -19.6999, "steps count": 1000, "duration": 937}
> {"episode": 23, "reward": 83.0000, "steps count": 456, "duration": 532}
> {"episode": 24, "reward": -13.9998, "steps count": 1000, "duration": 1236}
> {"episode": 25, "reward": 25.9003, "steps count": 1000, "duration": 1264}
> {"episode": 26, "reward": 1.2002, "steps count": 1000, "duration": 1349}
> {"episode": 27, "reward": 3.1000, "steps count": 1000, "duration": 1364}
> {"episode": 28, "reward": -6.4000, "steps count": 1000, "duration": 1392}
> {"episode": 29, "reward": -4.4998, "steps count": 1000, "duration": 1444}
> {"episode": 30, "reward": 3.1000, "steps count": 1000, "duration": 1611}

thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting

---

num_episodes: 40,
// memory_size: 8192, // must be set in dqn_model.rs with the MEMORY_SIZE constant
// max_steps: 700, // must be set in environment.rs with the MAX_STEPS constant
dense_size: 256, // neural network complexity
eps_start: 0.9, // epsilon initial value (0.9 => more exploration)
eps_end: 0.05,
eps_decay: 3000.0,

{"episode": 0, "reward": -1256.1014, "steps count": 700, "duration": 9}
{"episode": 1, "reward": -1256.1013, "steps count": 700, "duration": 20}
{"episode": 2, "reward": -1256.1014, "steps count": 700, "duration": 31}
{"episode": 3, "reward": -1258.7015, "steps count": 700, "duration": 44}
{"episode": 4, "reward": -1206.8009, "steps count": 700, "duration": 56}
{"episode": 5, "reward": -473.2974, "steps count": 700, "duration": 68}
{"episode": 6, "reward": -285.2984, "steps count": 700, "duration": 82}
{"episode": 7, "reward": -332.6987, "steps count": 700, "duration": 103}
{"episode": 8, "reward": -359.2984, "steps count": 700, "duration": 114}
{"episode": 9, "reward": -118.7008, "steps count": 700, "duration": 125}
{"episode": 10, "reward": -83.9004, "steps count": 700, "duration": 144}
{"episode": 11, "reward": -68.7006, "steps count": 700, "duration": 165}
{"episode": 12, "reward": -49.7002, "steps count": 700, "duration": 180}
{"episode": 13, "reward": -68.7002, "steps count": 700, "duration": 204}
{"episode": 14, "reward": -38.3001, "steps count": 700, "duration": 223}
{"episode": 15, "reward": -19.2999, "steps count": 700, "duration": 240}
{"episode": 16, "reward": -19.1998, "steps count": 700, "duration": 254}
{"episode": 17, "reward": -21.1999, "steps count": 700, "duration": 250}
{"episode": 18, "reward": -26.8998, "steps count": 700, "duration": 280}
{"episode": 19, "reward": -11.6999, "steps count": 700, "duration": 301}
{"episode": 20, "reward": -13.5998, "steps count": 700, "duration": 317}
{"episode": 21, "reward": 5.4000, "steps count": 700, "duration": 334}
{"episode": 22, "reward": 3.5000, "steps count": 700, "duration": 353}
{"episode": 23, "reward": 13.0000, "steps count": 700, "duration": 374}
{"episode": 24, "reward": 7.3001, "steps count": 700, "duration": 391}
{"episode": 25, "reward": -4.1000, "steps count": 700, "duration": 408}
{"episode": 26, "reward": -17.3998, "steps count": 700, "duration": 437}
{"episode": 27, "reward": 11.1001, "steps count": 700, "duration": 480}
{"episode": 28, "reward": -4.1000, "steps count": 700, "duration": 505}
{"episode": 29, "reward": -13.5999, "steps count": 700, "duration": 522}
{"episode": 30, "reward": -0.3000, "steps count": 700, "duration": 540}
{"episode": 31, "reward": -15.4998, "steps count": 700, "duration": 572}
{"episode": 32, "reward": 14.9001, "steps count": 700, "duration": 630}
{"episode": 33, "reward": -4.1000, "steps count": 700, "duration": 729}
{"episode": 34, "reward": 5.4000, "steps count": 700, "duration": 777}
{"episode": 35, "reward": 7.3000, "steps count": 700, "duration": 748}
{"episode": 36, "reward": 9.2001, "steps count": 700, "duration": 767}
{"episode": 37, "reward": 13.0001, "steps count": 700, "duration": 791}
{"episode": 38, "reward": -13.5999, "steps count": 700, "duration": 813}
{"episode": 39, "reward": 26.3002, "steps count": 700, "duration": 838}

> Sauvegarde du modèle de validation
> Modèle de validation sauvegardé : models/burn_dqn_50_model.mpk
> Chargement du modèle pour test
> Chargement du modèle depuis : models/burn_dqn_50_model.mpk
> Test avec le modèle chargé
> Episode terminé. Récompense totale: 70.00, Étapes: 700
