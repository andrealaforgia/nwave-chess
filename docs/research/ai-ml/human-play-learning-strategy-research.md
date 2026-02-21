# Learning Strategy for a Chess Program That Learns by Playing Against Humans

**Research Date:** 2026-02-20
**Research Scope:** How to build a chess program that visibly improves by playing against a single human opponent, with practical strategies for extreme data scarcity (tens to hundreds of games, not millions).
**Sources Consulted:** 30+
**Confidence Distribution:** High (40%), Medium (45%), Low (15%)
**Prerequisite Reading:** [Self-Learning Chess Comprehensive Research](./self-learning-chess-comprehensive-research.md)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [The Fundamental Challenge: Data Scarcity](#2-the-fundamental-challenge-data-scarcity)
3. [Historical Precedents: Programs That Learned from Human Play](#3-historical-precedents-programs-that-learned-from-human-play)
4. [TD-Leaf(lambda): The Proven Algorithm for Few-Game Learning](#4-td-leaflambda-the-proven-algorithm-for-few-game-learning)
5. [What Should the Learning Signal Be?](#5-what-should-the-learning-signal-be)
6. [Architecture for Sparse Data](#6-architecture-for-sparse-data)
7. [Hybrid Strategy: Human Games + Self-Play Augmentation](#7-hybrid-strategy-human-games--self-play-augmentation)
8. [Concrete Learning Strategy Design](#8-concrete-learning-strategy-design)
9. [Recommended Approach: Step-by-Step Implementation Plan](#9-recommended-approach-step-by-step-implementation-plan)
10. [Comparison of Viable Approaches](#10-comparison-of-viable-approaches)
11. [Expected Improvement Trajectory](#11-expected-improvement-trajectory)
12. [Knowledge Gaps](#12-knowledge-gaps)
13. [Source Analysis](#13-source-analysis)
14. [References](#14-references)

---

## 1. Executive Summary

Building a chess program that visibly improves by playing against a human is a fundamentally different problem from AlphaZero-style self-play training. The constraint is not compute -- it is **data**. A human plays 2-10 games per day, each lasting 10-60 minutes. The program must extract maximum learning from every single game.

**Key findings:**

- **TD-Leaf(lambda) is the proven approach for this exact problem.** KnightCap (1998) improved from 1650 to 2150 Elo in just 308 games on the Free Internet Chess Server. FUSc# improved by 350+ Elo in only 119 games. Both used hand-crafted evaluation features with TD-Leaf weight updates -- no deep neural network required. [Confidence: High]

- **Online play against humans outperforms self-play for TD learning.** The KnightCap authors found that self-play was ineffective because the engine always played the same moves, creating insufficient diversity. Human opponents provide varied strategies, unexpected moves, and escalating difficulty as the engine's rating increases. [Confidence: High]

- **Hand-crafted features dramatically outperform raw neural networks when data is scarce.** KnightCap used 5,872 features (piece values, piece-square tables, mobility, king safety). Giraffe used 363 features. Both achieved strong play. A deep neural network learning from raw board state needs orders of magnitude more data. [Confidence: High]

- **The recommended hybrid strategy combines three elements:** (1) a baseline evaluation bootstrapped from known chess principles, (2) TD-Leaf(lambda) weight updates after each human game, and (3) self-play augmentation between human games to amplify the learning signal. [Confidence: Medium]

- **Visible improvement is achievable within 50-150 games** if starting from a reasonable baseline (not random). KnightCap gained ~500 Elo in 308 games; FUSc# gained ~350 Elo in 119 games. A well-designed system should show noticeable improvement to the human player within weeks of regular play. [Confidence: Medium]

- **The minimum viable learning loop is surprisingly simple:** after each game, extract positions, compute TD errors between consecutive leaf evaluations, update feature weights via gradient descent, and optionally run a batch of self-play games using the updated weights. [Confidence: High]

---

## 2. The Fundamental Challenge: Data Scarcity

### 2.1 Quantifying the Data Problem

| Training Paradigm | Games Available | Positions per Day | Time to 100K games |
|-------------------|----------------|-------------------|---------------------|
| AlphaZero (5,000 TPUs) | ~44 million/day | ~2.2 billion | ~2 hours |
| Leela Chess Zero (volunteers) | ~1 million/day | ~50 million | ~4 days |
| Single GPU self-play | ~1,000-1,500/day | ~50,000-75,000 | ~2-3 months |
| **Human opponent (casual)** | **2-5/day** | **100-250** | **55-137 years** |
| **Human opponent (dedicated)** | **5-10/day** | **250-500** | **27-55 years** |

The data gap between human play and self-play is **three to six orders of magnitude**. This is not a problem that can be solved by simply running AlphaZero with fewer games. It requires fundamentally different algorithms optimized for sample efficiency.

**Sources:** [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [Leela Chess Zero](https://lczero.org/), [Baxter et al. - KnightCap](https://arxiv.org/abs/cs/9901002)

### 2.2 Why Standard Approaches Fail with Sparse Data

**AlphaZero-style training fails** because:
- The policy network needs thousands of games to learn basic move patterns
- MCTS visit counts require many games before the tree statistics become meaningful
- The neural network overfits catastrophically on just a few hundred games
- Exploration noise in a small sample leads to erratic rather than productive learning

**Pure value-based RL (DQN, etc.) fails** because:
- The enormous state space of chess (10^44 positions) means a few hundred games barely scratches the surface
- Replay buffers from just a few games provide almost no meaningful diversity
- The agent cannot distinguish signal from noise with so few data points

**What works** is algorithms that:
1. Start with strong priors (hand-crafted features encoding chess knowledge)
2. Update those priors incrementally based on game outcomes
3. Use temporal difference signals to learn from every move, not just game outcomes
4. Leverage the structure of minimax search to amplify learning per position

**Sources:** [Chessprogramming Wiki - Temporal Difference Learning](https://www.chessprogramming.org/Temporal_Difference_Learning), [Baxter et al., Machine Learning 2000](https://link.springer.com/article/10.1023/A:1007634325138)

---

## 3. Historical Precedents: Programs That Learned from Human Play

### 3.1 KnightCap (1998) -- The Gold Standard

KnightCap is the most important historical precedent for this project. It is a chess program that learned by playing against humans on the Free Internet Chess Server (FICS), improving dramatically in just days.

| Aspect | Detail |
|--------|--------|
| **Authors** | Jonathan Baxter, Andrew Tridgell, Lex Weaver (ANU) |
| **Language** | C |
| **Algorithm** | TD-Leaf(lambda) |
| **Evaluation features** | 5,872 parameters across 4 game stages (Opening, Middle, Ending, Mating) |
| **Feature types** | Piece values, piece-square tables (64 per piece), board control, hung/trapped/immobile pieces, mobility, asymmetric evaluation |
| **Search** | Iterative deepening, parallel MTD(f), null move pruning, razoring |
| **Learning rate (alpha)** | Configurable via TD_ALPHA in source |
| **Lambda** | Configurable via TD_LAMBDA in source |
| **Weight update frequency** | Every 10 games (configurable via MAX_GAMES) |
| **Sigmoid function** | tanh with 0.25 equivalent to one pawn advantage |
| **Rating improvement** | 1650 to 2150 Elo (500 points) |
| **Games required** | 308 games |
| **Time required** | 3 days |
| **Training method** | Online play against human and computer opponents on FICS |

**Key insight: why online play beat self-play.** The KnightCap authors explicitly reported that self-play was ineffective. When the engine played itself, it always chose the same moves, creating a narrow, repetitive training distribution. Online play against diverse human opponents provided:
- **Opponent diversity:** Every opponent played differently, exposing the engine to varied positions
- **Escalating difficulty:** As the engine's rating improved, it was matched against stronger opponents
- **Genuine surprise:** Human moves included creative and unexpected ideas the engine would never generate in self-play

**Treatment of opponent moves:** KnightCap applied an asymmetric learning rule. Negative temporal differences (positions that turned out worse than expected) were always used for learning. However, positive temporal differences (positions that turned out better than expected) were zeroed out **unless** the engine had correctly predicted the opponent's move. This avoided learning from opponent blunders -- the engine would not develop an evaluation function that assumes opponents will make mistakes.

**Sources:** [Baxter et al. - KnightCap arXiv](https://arxiv.org/abs/cs/9901002), [Baxter et al., Machine Learning 2000](https://link.springer.com/article/10.1023/A:1007634325138), [KnightCap - Chessprogramming Wiki](https://www.chessprogramming.org/KnightCap), [KnightCap Source Code - GitHub](https://github.com/aiftwn/KnightCap)

### 3.2 FUSc# (2002-2008) -- Confirming KnightCap's Results

FUSc# was an experimental chess engine developed at the Free University of Berlin that confirmed and extended KnightCap's approach.

| Aspect | Detail |
|--------|--------|
| **Authors** | Marco Block et al., AI-Game Programming Group, FU Berlin |
| **Language** | C# |
| **Algorithm** | TD-Leaf(lambda) |
| **Position classification** | 33 types (32 middlegame + 1 endgame) based on king placement and queen presence |
| **Features per type** | 1,706 positional coefficients |
| **Total parameters** | 56,298 coefficients |
| **Rating improvement** | 1800 to 2016 Elo (~216 points) |
| **Games for training** | 72 training games out of 119 played |
| **Training method** | Online play on chess servers |

**Key contribution:** FUSc# demonstrated that classifying positions into types (by king placement, queen presence) and maintaining separate coefficient vectors for each type improved learning. This meant that the engine could learn, for example, that a knight on d5 is more valuable when queens are on the board than when they are off. However, the authors noted a risk of "evaluation discontinuity" at type boundaries.

**Sources:** [Block, "Using Reinforcement Learning in Chess Engines," 2008](http://page.mi.fu-berlin.de/block/concibe2008.pdf), [FUSCsharp - Chessprogramming Wiki](https://www.chessprogramming.org/FUSCsharp)

### 3.3 Giraffe (2015) -- Deep RL with TD-Leaf

Giraffe bridged the gap between classical TD-Leaf and modern deep learning.

| Aspect | Detail |
|--------|--------|
| **Author** | Matthew Lai (Imperial College London) |
| **Algorithm** | TD-Leaf(lambda) with deep neural network |
| **Architecture** | 3-layer network (2 hidden + output), ReLU activation, tanh output |
| **Input features** | 363 total: position-level (5), piece lists with coordinates, sliding piece mobility, attack/defense maps |
| **Lambda** | 0.7 |
| **Learning rate (alpha)** | 1.0 (with AdaDelta for per-parameter adaptive rates) |
| **Loss function** | L1 (chosen over L2 due to outlier-rich TD errors) |
| **Training positions** | 5 million starting positions, ~175 million via self-play from those positions |
| **Training method** | Self-play from database positions (not online play against humans) |
| **Training time** | 72 hours on 2x 10-core Xeon CPUs |
| **Strength** | FIDE International Master level (~2400 Elo) |

**Key insight for sparse data:** Giraffe used AdaDelta optimizer with per-parameter adaptive learning rates. This meant that "nodes that are rarely activated retain higher learning rates to make best use of limited activations" -- a critical property for learning from few games where many features appear infrequently.

**Bootstrapping strategy:** Rather than starting from random weights, Giraffe initialized from "a very simple evaluation function containing only very basic material knowledge." This bootstrapping took only seconds and positioned the network near reasonable parameter values, dramatically accelerating convergence.

**Sources:** [Lai, "Giraffe: Using Deep Reinforcement Learning to Play Chess," 2015](https://arxiv.org/abs/1509.01549), [Giraffe - ar5iv](https://ar5iv.labs.arxiv.org/html/1509.01549), [Giraffe - Chessprogramming Wiki](https://www.chessprogramming.org/Giraffe)

### 3.4 Maia (2020-2024) -- Human-Like Play from Human Games

While Maia is not exactly a "learning from human play" system in the RL sense, it is highly relevant to this project.

| Aspect | Detail |
|--------|--------|
| **Authors** | McIlroy-Young et al. (Microsoft Research / U. Toronto) |
| **Purpose** | Predict human moves at specific skill levels, not play optimally |
| **Training data** | Millions of human games from Lichess |
| **Architecture** | Neural network (Leela Chess Zero architecture) |
| **Skill-level models** | 9 separate models targeting Lichess 1100-1900 |
| **Maia-2 (2024)** | Single model that adapts to any skill level in real-time |
| **Personalization** | Can mirror individual play style from just 20 games |
| **Player identification** | 86% accuracy at identifying a specific player from 10-game sets |

**Key insight for this project:** Maia demonstrates that a meaningful model of an individual player's behavior can be built from as few as 20 games. This suggests that learning from a single human opponent is feasible -- the model can adapt to that specific player's style and level.

**Sources:** [Maia Chess](https://www.maiachess.com/), [McIlroy-Young et al., KDD 2020](https://www.cs.toronto.edu/~ashton/pubs/maia-kdd2020.pdf), [Maia-2, NeurIPS 2024](https://www.cs.toronto.edu/~ashton/pubs/maia2-neurips2024.pdf), [Microsoft Research - Maia](https://www.microsoft.com/en-us/research/project/project-maia/)

---

## 4. TD-Leaf(lambda): The Proven Algorithm for Few-Game Learning

### 4.1 Why TD-Leaf(lambda) Is the Right Choice

TD-Leaf(lambda) is specifically designed for games with minimax search. It solves two critical problems:

1. **Standard TD(lambda) evaluates root positions**, but in chess with deep search, the root evaluation is dominated by the search tree, not the evaluation function. Updating based on root values barely changes the weights.

2. **TD-Leaf instead evaluates the leaf nodes of the principal variation** -- the positions at the bottom of the search tree where the evaluation function is actually applied. This targets the exact weights that matter.

**Sources:** [Baxter et al., "TDLeaf(lambda)," 1999](https://arxiv.org/abs/cs/9901001), [Chessprogramming Wiki - Temporal Difference Learning](https://www.chessprogramming.org/Temporal_Difference_Learning)

### 4.2 The Mathematical Framework

The TD-Leaf(lambda) update rule:

```
w = w + alpha * SUM(t=1 to N-1) [ gradient_J(x_t, w) * SUM(j=t to N-1) [ lambda^(j-t) * d_j ] ]
```

Where:
- `w` = weight vector of the evaluation function
- `alpha` = learning rate
- `x_t` = leaf position of the principal variation at move t
- `J(x_t, w)` = evaluation of the leaf position (after sigmoid transformation)
- `gradient_J(x_t, w)` = gradient of the evaluation with respect to weights
- `d_j` = temporal difference: `J(x_{j+1}, w) - J(x_j, w)`
- `lambda` = trace decay parameter (0 to 1)
- `N` = number of moves in the game

**What lambda controls:**
- `lambda = 0`: Each position is only compared to the next position (bootstrapping). Faster learning but may propagate errors.
- `lambda = 1`: Each position is compared to the actual game outcome (Monte Carlo). Slower but more accurate.
- `lambda = 0.7`: A common middle ground used by Giraffe. Credits the move immediately before a score change most heavily, with exponentially decaying credit for earlier moves.

**The sigmoid transformation** converts the raw evaluation score (in centipawns) to a winning probability in the range [-1, +1]:

```
J(x) = tanh(eval(x) / c)
```

Where `c` is a scaling constant. KnightCap used `c` such that 0.25 corresponds to a one-pawn advantage. This ensures that:
- The gradient is largest near equal positions (where small evaluation differences matter most)
- The gradient vanishes for decisive advantages (no need to refine evaluation of won positions)

**Sources:** [Baxter et al., "TDLeaf(lambda)," 1999](https://arxiv.org/abs/cs/9901001), [Giraffe, Lai 2015](https://arxiv.org/abs/1509.01549), [Chessprogramming Wiki - TD Learning](https://www.chessprogramming.org/Temporal_Difference_Learning)

### 4.3 Practical Implementation of TD-Leaf(lambda)

Here is a concrete step-by-step implementation:

**During the game (data collection):**

```
For each move in the game:
    1. Run minimax search (alpha-beta) to chosen depth
    2. Record the leaf position of the principal variation (PV)
    3. Record the leaf evaluation J(leaf_position, w)
    4. Record the gradient of the evaluation: dJ/dw for each weight
    5. Record the move played (by both sides)
```

**After the game (weight update):**

```
Given: sequence of leaf evaluations J_1, J_2, ..., J_N and final result z

1. Compute temporal differences:
   d_t = J_{t+1} - J_t  for t = 1, ..., N-2
   d_{N-1} = z - J_{N-1}  (use actual game result for the last move)

2. Compute weighted TD errors for each position:
   delta_t = SUM(j=t to N-1) [ lambda^(j-t) * d_j ]

3. Compute total gradient:
   total_gradient = SUM(t=1 to N-1) [ gradient_J(x_t, w) * delta_t ]

4. Update weights:
   w = w + alpha * total_gradient
```

**Critical implementation details:**
- Collect data for **both sides** (your moves and the opponent's moves provide learning signals)
- Use the **leaf evaluation**, not the root evaluation
- Store gradients during search to avoid recomputation
- Apply the KnightCap trick: zero out positive temporal differences when you did not predict the opponent's move (to avoid learning from blunders)

**Sources:** [KnightCap Source Code](https://github.com/aiftwn/KnightCap), [Baxter et al., "TDLeaf(lambda)"](https://arxiv.org/abs/cs/9901001), [Giraffe](https://ar5iv.labs.arxiv.org/html/1509.01549)

### 4.4 Parameter Choices for Few-Game Learning

Based on the historical precedents:

| Parameter | KnightCap | Giraffe | Recommended for This Project |
|-----------|-----------|---------|------------------------------|
| Lambda | Not published (see source) | 0.7 | **0.7** (balances credit assignment) |
| Learning rate | Not published | 1.0 (with AdaDelta) | **0.01-0.1** (with manual SGD) or **1.0** (with AdaDelta/Adam) |
| Search depth | Variable (iterative deepening) | Variable | **4-6 ply** (practical for real-time play) |
| Sigmoid scaling | 0.25 = 1 pawn | Not specified | **0.25 = 1 pawn** (KnightCap's proven choice) |
| Update frequency | Every 10 games | Continuous | **Every 1-5 games** (maximize learning speed) |
| Optimizer | Gradient descent | AdaDelta | **Adam** (best for sparse features) |

**On learning rate:** Don Beal's observation from the Chessprogramming Wiki is critical: "The learning rate has to be as large as one dares for fast learning, but low for stable values." For few-game learning, err on the side of a larger learning rate to show visible progress, accepting some evaluation instability.

**Sources:** [Chessprogramming Wiki - TD Learning](https://www.chessprogramming.org/Temporal_Difference_Learning), [Giraffe](https://arxiv.org/abs/1509.01549), [KnightCap](https://www.chessprogramming.org/KnightCap)

---

## 5. What Should the Learning Signal Be?

### 5.1 Signal Comparison

| Signal Type | Data per Game | Learning Efficiency | Complexity | Historical Evidence |
|-------------|--------------|---------------------|------------|---------------------|
| **Game outcome only** (win/loss/draw) | 1 data point | Very low -- must wait for game end | Simple | Monte Carlo methods; insufficient alone |
| **TD signal** (evaluation differences between consecutive positions) | ~40-80 data points (one per move) | **High** -- learns from every move | Medium | KnightCap, FUSc#, Giraffe all used this |
| **Move quality vs reference engine** | ~40-80 data points | Medium -- requires a reference engine | High | Supervised learning / distillation |
| **TD signal + game outcome** | ~40-80 data points + 1 | **Highest** -- combines per-move and terminal signals | Medium | This is what lambda controls in TD-Leaf |

### 5.2 Recommended: TD Signal with Terminal Correction

The TD-Leaf(lambda) signal is the clear winner for this use case. It gives you ~40-80 learning updates per game (one per move), compared to just 1 from the game outcome alone.

With lambda = 0.7, each position's update is a weighted combination:
- 30% weight on the immediate next position (TD bootstrapping)
- 21% weight on the position two moves later
- 14.7% weight on the position three moves later
- ... exponentially decaying ...
- Plus a contribution from the terminal game result

This means:
- You learn from every move, not just the final result
- Tactical errors are quickly identified (large TD differences)
- Strategic patterns are learned more slowly but more reliably (propagated from game outcomes)

### 5.3 Supplementary Signal: Learning from the Opponent's Moves

The opponent's moves provide additional learning signal:

1. **If the opponent plays a strong move** that the engine did not predict, the evaluation will shift significantly. This negative temporal difference (from the engine's perspective) teaches the engine that its previous evaluation was too optimistic.

2. **If the opponent plays a weak move** (blunder), the KnightCap approach is to **discard the positive TD signal** from that transition. The engine should not learn that its position was bad just because the opponent made it better through a blunder.

3. **The opponent's choice of opening and strategy** implicitly teaches the engine about what positions humans find playable or uncomfortable. Over many games, the engine learns which positions lead to human errors.

**Sources:** [Baxter et al., KnightCap](https://arxiv.org/abs/cs/9901002), [Chessprogramming Wiki - KnightCap](https://www.chessprogramming.org/KnightCap), [Chessprogramming Wiki - TD Learning](https://www.chessprogramming.org/Temporal_Difference_Learning)

---

## 6. Architecture for Sparse Data

### 6.1 Hand-Crafted Features vs Neural Networks

For this project, the answer is clear: **hand-crafted features with learnable weights**.

| Approach | Data Needed for Convergence | Interpretability | Implementation Effort |
|----------|-----------------------------|------------------|-----------------------|
| Deep CNN (AlphaZero-style) | Millions of games | Low | High |
| Shallow neural network (Giraffe-style) | ~175M positions (thousands of games) | Medium | Medium |
| **Hand-crafted features + linear weights** | **~100-500 games** | **High** | **Low-Medium** |
| NNUE-style (sparse features + shallow NN) | Millions of positions | Medium | Medium |

KnightCap achieved 2150 Elo with 5,872 hand-crafted feature weights and no neural network. The features encode decades of human chess knowledge -- piece values, piece-square tables, mobility, king safety -- and the TD algorithm only needs to learn the relative **weights** of these features, not discover the features themselves.

With a neural network on raw board state, the network must discover from scratch that, for example, centralized knights are better than corner knights. This discovery requires thousands of games. With a hand-crafted knight piece-square table, the initial weights already encode this; TD learning only needs to refine how much centralization matters relative to other factors.

**Sources:** [KnightCap - Chessprogramming Wiki](https://www.chessprogramming.org/KnightCap), [FUSCsharp - Chessprogramming Wiki](https://www.chessprogramming.org/FUSCsharp), [Giraffe](https://arxiv.org/abs/1509.01549), [Simplified Evaluation Function - Chessprogramming Wiki](https://www.chessprogramming.org/Simplified_Evaluation_Function)

### 6.2 Recommended Feature Set

Based on KnightCap and FUSc#, here is a recommended feature set organized by category:

**Tier 1: Essential (learn first, ~400 parameters)**

| Feature Category | Parameters | Description |
|-----------------|------------|-------------|
| Piece values | 6 x 2 phases = 12 | Value per piece type for middlegame and endgame |
| Piece-square tables | 6 x 64 x 2 phases = 768 (with symmetry: ~384) | Position bonus per piece per square, for middlegame and endgame |
| Game phase | 1 | Interpolation weight between middlegame and endgame tables |

**Tier 2: Important (add after basic learning works, ~200 parameters)**

| Feature Category | Parameters | Description |
|-----------------|------------|-------------|
| Pawn structure | ~50 | Isolated pawns, doubled pawns, passed pawns, pawn chains |
| King safety | ~30 | Pawn shield, open files near king, attacker count |
| Mobility | ~30 | Number of legal moves for each piece type |
| Bishop pair bonus | 2 | Bonus for having both bishops |
| Rook on open file | 4 | Bonus for rook on open/semi-open file |

**Tier 3: Refinement (add when Tier 2 is stable, ~200+ parameters)**

| Feature Category | Parameters | Description |
|-----------------|------------|-------------|
| Piece coordination | ~50 | Knight outposts, rook on 7th rank, connected rooks |
| Threats | ~30 | Hanging pieces, trapped pieces, pins, forks (detected) |
| Space | ~20 | Control of central squares, space advantage |
| Tempo | ~10 | Development in opening, initiative |

**Starting point:** Use the Simplified Evaluation Function from the Chessprogramming Wiki as initial weights. This provides piece values (P=100, N=320, B=330, R=500, Q=900, K=20000 centipawns) and piece-square tables for all pieces. These values are well-tested and provide a reasonable starting evaluation that can play semi-decent chess before any learning occurs.

**Sources:** [Simplified Evaluation Function - Chessprogramming Wiki](https://www.chessprogramming.org/Simplified_Evaluation_Function), [Piece-Square Tables - Chessprogramming Wiki](https://www.chessprogramming.org/Piece-Square_Tables), [Evaluation - Chessprogramming Wiki](https://www.chessprogramming.org/Evaluation), [KnightCap](https://www.chessprogramming.org/KnightCap)

### 6.3 Preventing Catastrophic Forgetting with Sparse Data

With only a few games, the risk of catastrophic forgetting is significant: a single unusual game could drastically shift weights in a wrong direction.

**Mitigation strategies (ordered by importance):**

1. **Regularization toward initial weights.** Add a penalty term that pulls weights back toward their starting values: `loss += lambda_reg * ||w - w_initial||^2`. This is analogous to Elastic Weight Consolidation (EWC), where important weights are anchored to their previous values. In practice, a simpler L2 regularization toward the initial hand-crafted weights works well.

2. **Bounded weight updates.** Clip the maximum change per weight per update. For piece values, never allow a pawn to be worth more than a knight or a knight to be worth more than a rook in a single update.

3. **Rolling average / exponential moving average.** Instead of using the latest weights directly, maintain a moving average: `w_play = beta * w_play + (1 - beta) * w_trained`. This smooths out noisy updates.

4. **Replay buffer of all past games.** Keep every game played and periodically retrain or fine-tune on the entire history, not just the latest game. With only hundreds of games, this is computationally trivial.

5. **Separate learning rates by feature category.** Piece values should change slowly (learning rate 0.001). Piece-square tables can change faster (learning rate 0.01). Tactical features change fastest (learning rate 0.1). This prevents a single game from destroying fundamental piece valuations.

**Sources:** [Kirkpatrick et al., "Overcoming Catastrophic Forgetting," PNAS 2017](https://www.pnas.org/doi/10.1073/pnas.1611835114), [Chessprogramming Wiki - TD Learning](https://www.chessprogramming.org/Temporal_Difference_Learning), [IBM - Catastrophic Forgetting](https://www.ibm.com/think/topics/catastrophic-forgetting)

### 6.4 Why Not a Neural Network?

A neural network is not recommended as the **primary** architecture for this project due to the data constraint. However, a **hybrid** approach is possible for future enhancement:

- **Phase 1 (0-200 games):** Use hand-crafted features with TD-Leaf. Maximum learning per game.
- **Phase 2 (200-1000 games):** Optionally add a small neural network (1-2 hidden layers, 64-128 units) that takes the hand-crafted features as input and outputs a refined evaluation. Train this network using the game history as a dataset.
- **Phase 3 (1000+ games):** The neural network can potentially replace some hand-crafted features, learning non-linear interactions between features that the linear model cannot capture.

This phased approach ensures visible progress from the start while allowing for increased sophistication as data accumulates.

---

## 7. Hybrid Strategy: Human Games + Self-Play Augmentation

### 7.1 The Core Idea

Human games are **anchor experiences** -- high-quality, diverse learning signals that reveal weaknesses in the evaluation function. Between human games, the engine can play itself to generate additional training data that explores the consequences of what it learned.

```
Human Game (high quality, diverse)     Self-Play Games (lower quality, more volume)
         |                                      |
         v                                      v
    [TD-Leaf update]  ---->  [Updated weights]  ---->  [Self-play with new weights]
                                                              |
                                                              v
                                                    [TD-Leaf update from self-play]
                                                              |
                                                              v
                                                    [Further refined weights]
                                                              |
                                                              v
                                                    [Play next human game]
```

### 7.2 How to Weight Human vs Self-Play Data

The problem with self-play (as noted by the KnightCap authors) is that it produces narrow, repetitive data. However, **targeted** self-play can be valuable.

**Recommended weighting strategy:**

| Data Source | Weight | Rationale |
|------------|--------|-----------|
| Human games | 1.0x | Full learning rate; most diverse and informative |
| Self-play from human-game positions | 0.5x | Half learning rate; starts from known-interesting positions |
| Random self-play | 0.1x-0.3x | Low learning rate; prevents overfitting to human opponent's style |

**Targeted self-play from human-game positions:**
1. After a human game, identify the 5-10 positions where the evaluation changed most dramatically (highest TD errors)
2. Start self-play games from each of those positions
3. Play 5-10 self-play games from each position, varying the opening moves slightly
4. This generates 25-100 additional games that explore the specific areas where the engine's evaluation was most wrong

### 7.3 Self-Play Between Human Games: Practical Schedule

Assuming the human plays 2-3 games per day:

```
Day 1:
  Morning: Human plays Game 1
  -> Engine updates weights
  -> Engine plays 20-50 self-play games from Game 1's critical positions
  -> Engine updates weights again

  Evening: Human plays Game 2
  -> Engine updates weights
  -> Engine plays 20-50 self-play games from Game 2's critical positions
  -> Engine updates weights again

  Overnight: Engine plays 100-200 general self-play games with added opening randomization
  -> Engine updates weights with low learning rate (0.1x)

Day 2:
  Human plays with the improved engine
  ...
```

This schedule generates 140-300 games per day: 2-3 from human play and the rest from self-play. The human games steer the learning, and self-play amplifies it.

### 7.4 Data Augmentation: Board Symmetry

Chess has a vertical axis of symmetry: a position mirrored left-to-right is strategically equivalent (with minor exceptions around castling). This can double the effective training data:

For each position, create a mirrored version by:
1. Flip the board left-to-right (a-file becomes h-file, etc.)
2. Adjust castling rights accordingly
3. The evaluation should be the same

This is standard practice in Leela Chess Zero and other engines. In our case, it effectively doubles the data from each human game.

**Sources:** [Leela Chess Zero - Board Flipping Discussion](https://github.com/glinscott/leela-chess/issues/25), [AlphaZero preprint](https://arxiv.org/pdf/1712.01815)

---

## 8. Concrete Learning Strategy Design

### 8.1 What Happens After Each Game Against the Human

This is the complete pipeline, step by step.

**Step 1: During the game -- collect data**

For each position (both the engine's and the human's moves):
```python
game_data = []
for each move in the game:
    position = current_board_state

    # Run search
    search_result = alpha_beta_search(position, depth=SEARCH_DEPTH)

    # Record the leaf of the principal variation
    pv_leaf = search_result.principal_variation[-1]  # deepest position in PV
    leaf_eval = evaluate(pv_leaf, weights)            # raw centipawn evaluation
    leaf_eval_sigmoid = tanh(leaf_eval / SIGMOID_SCALE)  # normalized to [-1, +1]

    # Record the gradient of the evaluation w.r.t. weights at the leaf
    gradient = compute_gradient(pv_leaf, weights)     # d(eval)/d(w) for each weight

    # Record whether we predicted the opponent's move
    predicted_opponent_move = (search_result.best_response == actual_opponent_move)

    game_data.append({
        'leaf_eval': leaf_eval_sigmoid,
        'gradient': gradient,
        'predicted_opponent': predicted_opponent_move,
        'move_number': move_number,
        'fen': position.fen()
    })

# Record game result
game_result = +1 (win), -1 (loss), or 0 (draw)  # from engine's perspective
```

**Step 2: After the game -- compute TD errors**

```python
N = len(game_data)
lambda_val = 0.7

# Compute temporal differences
td_errors = []
for t in range(N - 1):
    d_t = game_data[t + 1]['leaf_eval'] - game_data[t]['leaf_eval']

    # KnightCap trick: zero out positive TD errors when opponent blundered
    if d_t > 0 and not game_data[t]['predicted_opponent']:
        d_t = 0

    td_errors.append(d_t)

# Terminal correction: last position vs actual result
td_errors.append(game_result - game_data[N - 1]['leaf_eval'])

# Compute lambda-weighted cumulative TD errors
deltas = [0] * N
for t in range(N - 1, -1, -1):
    running_sum = 0
    for j in range(t, N):
        running_sum += (lambda_val ** (j - t)) * td_errors[j]
    deltas[t] = running_sum
```

**Step 3: Update weights**

```python
total_gradient = zero_vector(num_weights)

for t in range(N):
    total_gradient += game_data[t]['gradient'] * deltas[t]

# Apply regularization toward initial weights
regularization = LAMBDA_REG * (weights - initial_weights)

# Update
weights = weights + LEARNING_RATE * total_gradient - regularization

# Clip extreme changes
for i in range(num_weights):
    max_change = MAX_WEIGHT_CHANGE[category_of(i)]
    weights[i] = clip(weights[i],
                       old_weights[i] - max_change,
                       old_weights[i] + max_change)
```

**Step 4: Optional -- run self-play augmentation**

```python
# Find positions with highest TD errors (most learning potential)
critical_positions = sorted(
    range(len(td_errors)),
    key=lambda t: abs(td_errors[t]),
    reverse=True
)[:10]  # Top 10 most surprising positions

# Self-play from each critical position
for pos_idx in critical_positions:
    fen = game_data[pos_idx]['fen']
    for game_i in range(5):  # 5 self-play games per position
        self_play_data = play_self_play_game(
            start_fen=fen,
            weights=weights,
            noise=True  # Add Dirichlet noise for exploration
        )
        update_weights_from_game(self_play_data, learning_rate=0.5 * LEARNING_RATE)
```

**Step 5: Save and log**

```python
# Save updated weights
save_weights(weights, f"weights_after_game_{game_number}.dat")

# Log learning metrics
log({
    'game_number': game_number,
    'result': game_result,
    'avg_td_error': mean(abs(td_errors)),
    'max_td_error': max(abs(td_errors)),
    'weight_change_magnitude': norm(weights - old_weights),
    'estimated_elo': estimate_elo_from_weights(weights)  # if available
})
```

### 8.2 Data Extracted Per Game

| Data Item | Quantity | Purpose |
|-----------|----------|---------|
| Leaf evaluations (PV) | ~40-80 per game | TD error computation |
| Leaf position gradients | ~40-80 per game | Weight update direction |
| Opponent move predictions | ~40-80 per game | KnightCap blunder filter |
| Board positions (FEN) | ~40-80 per game | Self-play augmentation starting points |
| Game result | 1 per game | Terminal TD correction |
| Time per move | ~40-80 per game | Optional: weight moves by thinking time |
| Move chosen vs alternatives | ~40-80 per game | Optional: analysis of move quality |

### 8.3 Storage Requirements

Each game produces approximately:
- ~80 positions x (FEN string + gradient vector + evaluation + metadata)
- With 400 feature weights: ~80 x (100 bytes FEN + 400 x 4 bytes gradient + 4 bytes eval + 50 bytes metadata) = ~133 KB per game
- 1,000 games = ~130 MB total -- trivially small

---

## 9. Recommended Approach: Step-by-Step Implementation Plan

### 9.1 Phase 0: Build the Chess Engine Foundation (Before Learning)

Before adding any learning, build a basic chess engine:

1. **Board representation** using python-chess (or your own implementation)
2. **Alpha-beta search** with iterative deepening, at least 4-6 ply depth
3. **Move ordering** (captures first, killer moves, history heuristic) -- critical for search efficiency
4. **Static evaluation function** with the Simplified Evaluation Function weights from Chessprogramming Wiki
5. **UCI protocol** support so you can play against it in a GUI (Arena, CuteChess, or similar)

**Test:** The engine should play legal chess and have a style that is recognizably chess-like (develops pieces, castles, captures free material). Estimated strength: ~800-1200 Elo.

### 9.2 Phase 1: Add TD-Leaf Learning (The Core)

1. **Instrument the search** to record leaf positions and evaluations along the principal variation
2. **Implement gradient computation** for the evaluation function (the derivative of each feature's contribution with respect to its weight)
3. **Implement the TD-Leaf update rule** as described in Section 8.1
4. **Add the KnightCap blunder filter** (zero out positive TD errors when opponent's move was not predicted)
5. **Add weight persistence** (save/load weights to/from file)
6. **Add weight clipping and regularization** to prevent catastrophic forgetting

**Test:** Play 10-20 games against the engine. Verify that weights change after each game and that the changes are in reasonable directions (e.g., if the engine lost because of weak king safety, king safety weights should increase).

### 9.3 Phase 2: Add Self-Play Augmentation

1. **Implement a self-play loop** that takes a starting position and plays a game using the current weights
2. **Add position selection** to identify critical positions from human games (highest TD errors)
3. **Add the augmentation schedule** from Section 7.3
4. **Add exploration noise** (Dirichlet noise or epsilon-greedy move selection) to self-play to prevent repetitive games
5. **Use a lower learning rate** for self-play data (0.3x-0.5x of human game rate)

**Test:** Verify that self-play games generate diverse positions (not always the same game). Verify that weights continue to move in reasonable directions.

### 9.4 Phase 3: Add Measurement and Visibility

Making the learning visible to the human is critical for engagement.

1. **Track Elo estimate** by periodically playing against Stockfish at fixed levels (using the python-stockfish wrapper)
2. **Display learning metrics** after each game:
   - "I learned the most from move 23 (my evaluation changed by X centipawns)"
   - "My biggest weakness this game was king safety / pawn structure / piece activity"
   - "Estimated strength improvement since game 1: +N Elo"
3. **Save weight snapshots** and allow the human to play against older versions to see the difference
4. **Track feature weight evolution** over time (graph piece values, key piece-square values, king safety weight)

### 9.5 Phase 4: Refinements

1. **Add position type classification** (like FUSc#): separate weights for positions with/without queens, different king placements
2. **Add more evaluation features** (Tier 2 and Tier 3 from Section 6.2)
3. **Implement opening book learning** (remember which openings led to wins/losses, adjust opening play)
4. **Consider adding a small neural network** on top of features for non-linear learning (only after 500+ games)
5. **Implement the Texel tuning method** as a periodic batch optimization: use all accumulated games to do a full optimization of all weights simultaneously

**Sources:** [Texel's Tuning Method - Chessprogramming Wiki](https://www.chessprogramming.org/Texel's_Tuning_Method)

---

## 10. Comparison of Viable Approaches

### 10.1 Approach Comparison Matrix

| Approach | Games to Visible Improvement | Implementation Complexity | Max Potential Elo | Data Requirement | Best For |
|----------|------------------------------|--------------------------|-------------------|------------------|----------|
| **TD-Leaf + hand-crafted features** | **50-150 games** | **Medium** | **~2000-2200** | **Very low** | **This project** |
| TD-Leaf + shallow neural network (Giraffe-style) | 200-500 games | Medium-High | ~2400 | Low-Medium | If you want higher ceiling |
| AlphaZero-style (MCTS + deep NN) | 10,000+ games | Very High | ~3500+ | Very high | Not suitable for this project |
| NNUE-style (sparse features + shallow NN) | 1,000+ games | High | ~3000+ | Medium | If combined with self-play |
| Supervised learning on game outcomes | 100-300 games | Low | ~1500 | Low | Quick prototype |
| Pure Texel tuning on game results | 200-500 games | Low | ~1800 | Low-Medium | Batch optimization alternative |

### 10.2 Recommended Approach: TD-Leaf + Hand-Crafted Features + Self-Play Augmentation

**Why this wins for the stated requirements:**

1. **Fastest visible improvement:** KnightCap showed 500 Elo gain in 308 games, FUSc# showed 350 Elo gain in 119 games. No other approach has demonstrated comparable improvement with so few games.

2. **Proven track record:** This is not theoretical -- two independent research groups validated this approach with real online play against humans.

3. **Interpretable learning:** When the engine learns, you can see *what* it learned (king safety weight increased, knight centralization value increased, etc.). With a neural network, learning is a black box.

4. **Graceful degradation:** Even if learning produces some bad weight updates, the engine still plays reasonable chess because the feature structure encodes sound chess principles. A neural network with bad weights produces nonsensical play.

5. **Low compute requirement:** No GPU needed. Alpha-beta search with a linear evaluation function runs on any CPU. Self-play augmentation adds modest compute.

---

## 11. Expected Improvement Trajectory

### 11.1 Estimated Elo Progression

Based on KnightCap and FUSc# results, extrapolated to this project's constraints:

| Milestone | Games Required | Calendar Time (3 games/day) | What Changes |
|-----------|---------------|----------------------------|--------------|
| **Baseline** (before learning) | 0 | Day 0 | Hand-crafted evaluation; ~800-1200 Elo |
| **First noticeable improvement** | 10-30 games | 1-2 weeks | Piece values stabilize; gross blunders decrease |
| **Consistent improvement** | 50-100 games | 2-5 weeks | Piece-square tables refine; positional play improves |
| **Significant strength gain** | 100-200 games | 1-2 months | +200-400 Elo from baseline; human notices engine "understanding" openings better |
| **Approaching ceiling** | 300-500 games | 2-4 months | +400-600 Elo from baseline; diminishing returns begin |
| **Plateau** | 500+ games | 4+ months | Feature set limits further improvement; need more features or neural network |

[Confidence: Medium -- extrapolated from KnightCap and FUSc# with different conditions]

### 11.2 What the Human Will Notice

- **Games 1-10:** The engine plays consistently but makes positional errors. It may overvalue or undervalue certain piece placements.
- **Games 10-30:** The engine starts "remembering" what hurt it. If the human exploited weak king safety, the engine starts prioritizing king safety.
- **Games 30-100:** The engine develops a style adapted to the human opponent. It may develop strengths in areas the human is weak and vice versa.
- **Games 100-300:** The engine plays noticeably stronger. Piece values and positional understanding are significantly refined. The human may find it harder to win.
- **Games 300+:** Improvement slows. The linear feature model approaches its ceiling. The human may need to improve their own play to continue challenging the engine.

### 11.3 Factors That Accelerate or Slow Learning

**Accelerators:**
- Playing longer games (more positions per game)
- Playing diverse openings (exposes engine to varied positions)
- Self-play augmentation between human games
- Starting from good initial weights (Simplified Evaluation Function)
- Higher learning rate (at the cost of some instability)

**Decelerators:**
- Playing only blitz games (few positions, noisy data due to time pressure)
- Always playing the same opening (engine overfits to narrow position type)
- Human skill much stronger or weaker than engine (one-sided games provide less learning signal)
- Too-low learning rate (engine barely changes)
- Catastrophic forgetting due to insufficient regularization

---

## 12. Knowledge Gaps

### 12.1 Optimal Lambda and Learning Rate for Few-Game Online Learning

**Searched for:** Rigorous hyperparameter studies for TD-Leaf(lambda) in the specific regime of tens to hundreds of games.
**Found:** KnightCap and FUSc# report results but do not publish hyperparameter sweeps. Giraffe used lambda=0.7 but trained on millions of positions. No study directly addresses the few-hundred-game regime.
**Impact:** The recommended values (lambda=0.7, learning rate 0.01-0.1) are educated guesses based on adjacent evidence. Experimentation will be required.

### 12.2 Interaction Between Self-Play Augmentation and Online Learning

**Searched for:** Studies that systematically combine online play against humans with self-play augmentation in the same training pipeline.
**Found:** No published work addresses this hybrid approach for chess. KnightCap used only online play. Giraffe used only self-play. The hybrid strategy recommended here is an **original design** based on combining principles from both approaches.
**Impact:** The recommended weighting of human vs self-play data (Section 7.2) is a hypothesis, not established practice.

### 12.3 Long-Term Stability of TD-Leaf with Sparse Online Data

**Searched for:** Evidence of weight divergence or instability in TD-Leaf(lambda) when trained on very few games over extended periods.
**Found:** Don Beal's observation about the tradeoff between learning speed and stability is relevant but not specific. FUSc# noted evaluation discontinuity at position-type boundaries. No study tracks weight evolution over hundreds of games.
**Impact:** The regularization and weight-clipping strategies recommended in Section 6.3 are precautionary measures without empirical validation in this specific context.

### 12.4 Optimal Self-Play Volume and Strategy Between Human Games

**Searched for:** How many self-play games, and from which starting positions, optimally augment a small set of human games.
**Found:** No published work addresses this question. The recommendation of 20-50 self-play games per human game from critical positions is an original estimate.
**Impact:** The self-play augmentation schedule should be treated as a starting point subject to experimentation.

### 12.5 Whether Modern Optimizers (Adam, AdaDelta) Improve on SGD for TD-Leaf

**Searched for:** Comparison of optimizers for TD-Leaf weight updates in chess.
**Found:** Giraffe used AdaDelta and noted benefits for sparse feature activations. No systematic comparison exists.
**Impact:** Adam is recommended based on general ML knowledge about its effectiveness with sparse gradients, but this is not chess-specific evidence.

---

## 13. Source Analysis

| Source | Type | Reputation | Independence | Used For |
|--------|------|------------|-------------|----------|
| Baxter et al., "KnightCap" (1999/2000) | Peer-reviewed paper | Tier 1 | Primary source | Core algorithm, results, online vs self-play comparison |
| Baxter et al., "TDLeaf(lambda)" (1999) | Peer-reviewed paper | Tier 1 | Same authors as KnightCap | Mathematical framework for TD-Leaf |
| Lai, "Giraffe" (2015) | MSc Thesis / arXiv | Tier 2 | Independent researcher | Deep RL with TD-Leaf, architecture details, AdaDelta |
| Block, "Using RL in Chess Engines" (2008) | Conference paper | Tier 2 | Independent (FU Berlin) | FUSc# results, position classification |
| McIlroy-Young et al., "Maia" (2020/2024) | Peer-reviewed (KDD, NeurIPS) | Tier 1 | Independent (Microsoft/U.Toronto) | Personalized models from few games |
| Chessprogramming Wiki - TD Learning | Community wiki | Tier 2 | Community-maintained | Practical details, historical context |
| Chessprogramming Wiki - KnightCap | Community wiki | Tier 2 | Community-maintained | Technical details, source code analysis |
| Chessprogramming Wiki - FUSCsharp | Community wiki | Tier 2 | Community-maintained | FUSc# details, feature counts |
| Chessprogramming Wiki - Simplified Eval | Community wiki | Tier 2 | Community-maintained | Baseline evaluation function |
| Chessprogramming Wiki - Piece-Square Tables | Community wiki | Tier 2 | Community-maintained | Feature design |
| Chessprogramming Wiki - Texel Tuning | Community wiki | Tier 2 | Community-maintained | Batch optimization alternative |
| Chessprogramming Wiki - Evaluation | Community wiki | Tier 2 | Community-maintained | Feature design guidance |
| KnightCap GitHub Source | Source code | Tier 3 | Same as paper | Implementation verification |
| Silver et al., Science 2018 | Peer-reviewed paper | Tier 1 | Independent (DeepMind) | AlphaZero baseline comparison |
| Leela Chess Zero | Open-source project | Tier 1 | Community project | Data augmentation practices |
| Kirkpatrick et al., "EWC" PNAS 2017 | Peer-reviewed paper | Tier 1 | Independent (DeepMind) | Catastrophic forgetting prevention |
| Schaeffer et al., Prioritized Exp. Replay | Peer-reviewed paper | Tier 1 | Independent (DeepMind) | Replay buffer design |
| Ye et al., "EfficientZero" (2021) | Peer-reviewed (NeurIPS) | Tier 1 | Independent | Sample-efficient RL |
| Schrittwieser et al., "MuZero" (2020) | Peer-reviewed (Nature) | Tier 1 | Independent (DeepMind) | Model-based sample efficiency |

---

## 14. References

### Core Papers for This Project

1. Baxter, J., Tridgell, A., Weaver, L. "KnightCap: A chess program that learns by combining TD(lambda) with game-tree search." *arXiv* cs/9901002 (1999). [Link](https://arxiv.org/abs/cs/9901002)
2. Baxter, J., Tridgell, A., Weaver, L. "TDLeaf(lambda): Combining Temporal Difference Learning with Game-Tree Search." *arXiv* cs/9901001 (1999). [Link](https://arxiv.org/abs/cs/9901001)
3. Baxter, J., Tridgell, A., Weaver, L. "Learning to Play Chess Using Temporal Differences." *Machine Learning* 40 (2000). [Link](https://link.springer.com/article/10.1023/A:1007634325138)
4. Lai, M. "Giraffe: Using Deep Reinforcement Learning to Play Chess." *arXiv* 1509.01549 (2015). [Link](https://arxiv.org/abs/1509.01549)
5. Block, M. "Using Reinforcement Learning in Chess Engines." *ConCIBe* (2008). [Link](http://page.mi.fu-berlin.de/block/concibe2008.pdf)

### Human-Like Chess and Personalization

6. McIlroy-Young, R. et al. "Aligning Superhuman AI with Human Behavior: Chess as a Model System." *KDD* (2020). [Link](https://www.cs.toronto.edu/~ashton/pubs/maia-kdd2020.pdf)
7. Tang, Z. et al. "Maia-2: A Unified Model for Human-AI Alignment in Chess." *NeurIPS* (2024). [Link](https://www.cs.toronto.edu/~ashton/pubs/maia2-neurips2024.pdf)
8. McIlroy-Young, R. et al. "Learning Personalized Models of Human Behavior in Chess." *KDD* (2022). [Link](https://www.cs.toronto.edu/~ashton/pubs/maia-individual-kdd2022.pdf)

### Catastrophic Forgetting and Continual Learning

9. Kirkpatrick, J. et al. "Overcoming catastrophic forgetting in neural networks." *PNAS* 114.13 (2017). [Link](https://www.pnas.org/doi/10.1073/pnas.1611835114)

### Sample-Efficient RL

10. Ye, W. et al. "Mastering Atari Games with Limited Data." *NeurIPS* (2021). [Link](https://arxiv.org/abs/2111.00210)
11. Schrittwieser, J. et al. "Mastering Atari, Go, Chess and Shogi by Planning with a Learned Model." *Nature* 588 (2020). [Link](https://arxiv.org/abs/1911.08265)

### Evaluation Function Design

12. Simplified Evaluation Function. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/Simplified_Evaluation_Function)
13. Piece-Square Tables. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/Piece-Square_Tables)
14. Texel's Tuning Method. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/Texel's_Tuning_Method)
15. Evaluation. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/Evaluation)

### Temporal Difference Learning

16. Temporal Difference Learning. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/Temporal_Difference_Learning)
17. Sutton, R. "Learning to predict by the methods of temporal differences." *Machine Learning* 3.1 (1988). [Link](http://incompleteideas.net/papers/sutton-88-with-erratum.pdf)

### Source Code and Implementations

18. KnightCap Source Code. *GitHub*. [Link](https://github.com/aiftwn/KnightCap)
19. Maia Chess. [Link](https://www.maiachess.com/)
20. FUSCsharp. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/FUSCsharp)

### Supplementary

21. Silver, D. et al. "A general reinforcement learning algorithm that masters chess, shogi, and Go through self-play." *Science* 362.6419 (2018). [Link](https://www.science.org/doi/10.1126/science.aar6404)
22. Leela Chess Zero. [Link](https://lczero.org/)
23. Schaul, T. et al. "Prioritized Experience Replay." *ICLR* (2016). [Link](https://arxiv.org/abs/1511.05952)
24. KnightCap. *Chessprogramming Wiki*. [Link](https://www.chessprogramming.org/KnightCap)
25. Klein, D. "Neural Networks for Chess." *arXiv* 2209.01506 (2022). [Link](https://arxiv.org/abs/2209.01506)
