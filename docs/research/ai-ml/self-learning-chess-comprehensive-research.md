# Self-Learning Chess Program: Comprehensive Research

**Research Date:** 2026-02-19
**Research Scope:** How to build a chess program that learns purely from self-play, starting with only knowledge of legal moves.
**Sources Consulted:** 25+
**Confidence Distribution:** High (60%), Medium (30%), Low (10%)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [AlphaZero: The Landmark System](#2-alphazero-the-landmark-system)
3. [Reinforcement Learning for Board Games](#3-reinforcement-learning-for-board-games)
4. [Open-Source Implementations](#4-open-source-implementations)
5. [Practical Considerations for a Personal Project](#5-practical-considerations-for-a-personal-project)
6. [Alternative Approaches](#6-alternative-approaches)
7. [Technical Architecture Details](#7-technical-architecture-details)
8. [Key Challenges and Pitfalls](#8-key-challenges-and-pitfalls)
9. [Recommended Approach for a Personal Project](#9-recommended-approach-for-a-personal-project)
10. [Knowledge Gaps](#10-knowledge-gaps)
11. [Source Analysis](#11-source-analysis)
12. [References](#12-references)

---

## 1. Executive Summary

Building a chess program that learns purely from self-play is achievable, but the path you choose depends heavily on your available compute and patience. The spectrum runs from DeepMind's AlphaZero (thousands of TPUs, 9 hours to superhuman play) to budget implementations on a single consumer GPU (weeks to months for intermediate-level play). This research document maps the full landscape and provides concrete, actionable guidance.

**Key findings:**

- **AlphaZero proved** that a system with zero chess knowledge beyond legal moves can reach superhuman play through self-play and reinforcement learning. Its architecture (deep residual neural network + Monte Carlo Tree Search) is the gold standard. [Confidence: High]
- **Consumer hardware is viable** but requires significant compromises. Pure self-play RL on a single GPU is extremely slow; hybrid approaches (supervised pre-training + self-play fine-tuning) are far more practical. [Confidence: High]
- **Leela Chess Zero (lc0)** is the most successful open-source replication, but achieved its results through distributed volunteer computing equivalent to thousands of GPUs over years. [Confidence: High]
- **Reaching ~1200 Elo** (beginner-intermediate) is achievable on a single GPU in days to weeks with supervised learning on human games. Reaching ~2000 Elo (strong club player) via pure self-play on consumer hardware may take months. [Confidence: Medium]
- **Recent algorithmic advances** (search-contempt, 2025) promise to reduce the compute cost of self-play training by orders of magnitude, potentially making consumer-GPU training from zero more feasible. [Confidence: Medium]
- **Simpler alternatives exist**: TD-learning, NNUE-style shallow networks, and hybrid feature-based approaches offer faster paths to reasonable play with less compute. [Confidence: High]

---

## 2. AlphaZero: The Landmark System

### 2.1 Overview

AlphaZero, published by DeepMind in December 2017 (preprint) and December 2018 (Science), is a general-purpose reinforcement learning algorithm that mastered chess, shogi, and Go through self-play alone, with no domain knowledge beyond the rules of each game.

After just 4 hours of training on specialized hardware, AlphaZero exceeded the Elo rating of Stockfish 8 (the strongest traditional chess engine at the time). After 9 hours, it defeated Stockfish 8 in a 100-game match: 28 wins, 0 losses, 72 draws.

**Sources:** [AlphaZero - Wikipedia](https://en.wikipedia.org/wiki/AlphaZero), [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [DeepMind Blog](https://deepmind.google/blog/alphazero-shedding-new-light-on-chess-shogi-and-go/)

### 2.2 Architecture

AlphaZero integrates two components:

1. **Deep Neural Network** -- takes a board position as input and outputs:
   - A **policy vector** (probability distribution over all legal moves)
   - A **value scalar** (estimated probability of winning from this position, in range [-1, +1])

2. **Monte Carlo Tree Search (MCTS)** -- uses the neural network to evaluate positions and guide search, producing stronger move selections than the raw network alone.

#### Neural Network Specifics

| Component | Detail |
|-----------|--------|
| **Input encoding** | 8x8x119 tensor (119 planes per square) |
| **Input planes** | 6 piece types x 2 colors x 8 time steps (current + 7 history) = 96 planes, plus 23 auxiliary planes (castling rights, move counters, side to move, repetition) |
| **Body** | 1 convolutional layer + 19 residual blocks |
| **Filters per layer** | 256 filters, 3x3 kernel, stride 1 |
| **Residual block** | 2 convolutional layers + batch normalization + ReLU + skip connection |
| **Policy head** | Conv layer -> 73 filters -> fully connected -> softmax over 4,672 possible move encodings (64 source squares x 73 move types) |
| **Value head** | 1x1 conv -> 256 FC units -> ReLU -> 1 FC unit -> tanh output |
| **Total parameters** | Approximately 20-30 million |

**Sources:** [Chessprogramming Wiki - AlphaZero](https://www.chessprogramming.org/AlphaZero), [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [Stanford CS221 Section](https://web.stanford.edu/class/archive/cs/cs221/cs221.1196/sections/Section5.pdf)

### 2.3 MCTS Integration

AlphaZero uses a variant of UCT called **PUCT** (Predictor + Upper Confidence Bound for Trees):

```
SELECT child that maximizes: Q(s,a) + c_puct * P(s,a) * sqrt(N(s)) / (1 + N(s,a))
```

Where:
- `Q(s,a)` = mean value of action `a` from state `s` (exploitation)
- `P(s,a)` = prior probability from the policy network (neural guidance)
- `N(s)` = visit count of parent node
- `N(s,a)` = visit count of this action
- `c_puct` = exploration constant (AlphaZero uses ~1.0)

**Key insight:** AlphaZero searches only ~80,000 positions per second (vs. Stockfish's 70 million), but its neural network focuses search on the most promising variations, making each evaluation far more informative.

During training, AlphaZero runs **800 MCTS simulations per move**. The visit counts from MCTS are converted to a move probability distribution using a temperature parameter:
- **Temperature = 1** for the first 30 moves (encourages exploration)
- **Temperature -> 0** for remaining moves (exploits best moves found)

**Sources:** [Chessprogramming Wiki - AlphaZero](https://www.chessprogramming.org/AlphaZero), [Josh Varty - Alpha Zero and MCTS](https://joshvarty.github.io/AlphaZero/), [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404)

### 2.4 Training Methodology

The training loop is conceptually simple:

```
repeat:
    1. SELF-PLAY: Use current network + MCTS to play games against itself
       -> Collect (board_state, MCTS_policy, game_outcome) for every position
    2. TRAIN: Update network to:
       - Match the policy head output to MCTS visit count distribution
       - Match the value head output to the actual game result (+1, -1, 0)
    3. (No explicit model comparison -- network is updated continuously)
```

**Training parameters:**
- 700,000 training steps
- Mini-batch size: 4,096 positions
- Self-play generation: 5,000 first-generation TPUs
- Network training: 64 second-generation TPUs
- A first-generation TPU is roughly comparable to an NVIDIA Titan V GPU for inference

**Critical difference from AlphaGo Zero:** AlphaZero maintains a single continuously-updated network rather than comparing successive iterations and keeping the best one. This simplifies the training pipeline.

**Sources:** [AlphaZero - Wikipedia](https://en.wikipedia.org/wiki/AlphaZero), [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [DeepMind Blog](https://deepmind.google/blog/alphazero-shedding-new-light-on-chess-shogi-and-go/)

### 2.5 Key Innovations vs Traditional Engines

| Aspect | Traditional (Stockfish) | AlphaZero |
|--------|------------------------|-----------|
| Evaluation | Hand-crafted heuristics (thousands of lines of tuned code) | Learned neural network |
| Search | Alpha-beta with many extensions | MCTS guided by neural network |
| Positions/sec | ~70 million | ~80,000 |
| Knowledge | Decades of human chess expertise encoded | Zero chess knowledge beyond rules |
| Playing style | Precise, materialistic | Creative, positional, "human-like" |

**Sources:** [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [PNAS - Acquisition of Chess Knowledge in AlphaZero](https://www.pnas.org/doi/10.1073/pnas.2206625119), [Chess.com - What's Inside AlphaZero's Brain](https://www.chess.com/article/view/whats-inside-alphazeros-brain)

---

## 3. Reinforcement Learning for Board Games

### 3.1 Self-Play Reinforcement Learning

Self-play is the core mechanism by which a chess agent improves without external data. The agent plays both sides of a game, using its current policy. The outcome of the game (win/loss/draw) provides the reward signal. Over many iterations, the agent learns to play moves that lead to winning outcomes.

**The self-play loop:**
1. Agent plays a complete game against itself
2. Each position in the game is labeled with the final outcome
3. The neural network is trained to predict: (a) the moves chosen by the stronger search (MCTS), and (b) the game outcome
4. The updated network generates new self-play games
5. Repeat

**Sources:** [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [Emergent Mind - AlphaZero-Based System](https://www.emergentmind.com/topics/alphazero-based-system), [Medium - Self-Play RL for Strategic Games](https://medium.com/biased-algorithms/self-play-reinforcement-learning-for-strategic-games-886cf4b9baf8)

### 3.2 Policy Networks vs Value Networks

In the AlphaZero framework, a single neural network has two output "heads":

- **Policy head:** Outputs a probability distribution over possible moves. This guides the MCTS search toward promising branches. The policy is trained to match the visit count distribution produced by MCTS (which is a stronger estimator than the raw policy).

- **Value head:** Outputs a scalar in [-1, +1] estimating the expected game outcome from the current position. This replaces the traditional evaluation function and also replaces MCTS rollouts (random playouts to the end of the game).

**Why a unified network?** Sharing the body (residual tower) between both heads forces the network to learn a shared representation that captures both "what to do" (policy) and "how good is this position" (value). This shared representation is more data-efficient than training separate networks.

**Sources:** [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [Leiden University - Policy or Value?](https://liacs.leidenuniv.nl/~plaata1/papers/CoG2019.pdf), [Emergent Mind](https://www.emergentmind.com/topics/alphazero-based-system)

### 3.3 Monte Carlo Tree Search (MCTS)

MCTS builds a search tree incrementally by repeating four phases:

1. **Selection:** Starting from the root, select child nodes using PUCT until reaching an unexpanded node
2. **Expansion:** Add the new node to the tree
3. **Evaluation:** Use the neural network to get a value estimate (replacing random rollouts)
4. **Backpropagation:** Update visit counts and value estimates along the path back to the root

After the desired number of simulations (e.g., 800), the move is chosen based on visit counts at the root. The visit count distribution serves as an improved policy target for training.

**Why MCTS instead of alpha-beta?** MCTS naturally integrates with neural network evaluation and handles the uncertainty of learned evaluations. Alpha-beta search requires a reliable evaluation function and benefits from exact minimax values, which a neural network cannot guarantee.

**Sources:** [Monte Carlo Tree Search - Wikipedia](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search), [Josh Varty - Alpha Zero and MCTS](https://joshvarty.github.io/AlphaZero/), [Chessprogramming Wiki - AlphaZero](https://www.chessprogramming.org/AlphaZero)

### 3.4 Reward Signals in Chess

Chess provides a sparse reward signal:
- **+1** for a win
- **-1** for a loss
- **0** for a draw

This signal comes only at the end of the game, which can be 40-100+ moves. The value network learns to propagate this terminal reward backward through positions, effectively learning "which positions are winning."

**No intermediate rewards are hand-coded.** The agent must discover concepts like material advantage, king safety, and pawn structure purely through their correlation with game outcomes.

**Sources:** [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [Medium - RL in Chess](https://medium.com/@samgill1256/reinforcement-learning-in-chess-73d97fad96b3)

### 3.5 Temporal Difference Learning

TD learning is an older but still relevant approach where the agent learns to predict outcomes based on differences between successive predictions during a game. Rather than waiting for the game to end, the value function is updated at every move.

**Historical significance:**
- **TD-Gammon** (Gerald Tesauro, 1992): Used TD(lambda) to train a backgammon program to world-class level through self-play. This was the first major success of neural-network self-play in games.
- **KnightCap** (1998): Used TD-Leaf(lambda) -- a variant that integrates TD learning with minimax search -- to improve from ~1650 to ~2150 Elo in only 308 games and 3 days of play on internet chess servers.

TD learning is simpler to implement than full AlphaZero and can work with traditional alpha-beta search. It is a viable approach for a simpler self-learning chess project.

**Sources:** [Chessprogramming Wiki - Temporal Difference Learning](https://www.chessprogramming.org/Temporal_Difference_Learning), [Tesauro - TD-Gammon](https://www.bkgm.com/articles/tesauro/tdl.html), [Baxter et al. - Learning to Play Chess Using Temporal Differences](https://link.springer.com/article/10.1023/A:1007634325138)

---

## 4. Open-Source Implementations

### 4.1 Leela Chess Zero (lc0)

**The most successful open-source AlphaZero replication.**

| Aspect | Detail |
|--------|--------|
| **Website** | [lczero.org](https://lczero.org/) |
| **GitHub** | [LeelaChessZero/lc0](https://github.com/LeelaChessZero/lc0) |
| **Architecture** | Originally residual CNNs; switched to transformer-based architecture in 2022 |
| **Training** | Distributed volunteer computing; 2.5+ billion self-play games as of 2025 |
| **Input** | 112 planes of 8x8 |
| **Network sizes** | 10x128, 20x256, 24x320 (blocks x filters) |
| **Output** | 1,858 possible move actions + Win/Draw/Loss value head |
| **Strength** | Comparable to Stockfish (the world's strongest engine) |

**Training model:** Volunteers download the lc0 client, generate self-play games locally on their GPUs, and upload the data to a central server that trains the network. The project produces ~1 million games per day across all volunteers. A single contributor can generate 1,000-1,500 games per day on consumer hardware.

**Key insight for personal projects:** Lc0's superhuman strength is the result of years of distributed training across hundreds to thousands of GPUs. You cannot replicate this alone, but you can use their pre-trained networks or study their architecture.

As of November 2024, most Lc0 models in use are actually trained via **supervised learning** on data generated by previous RL runs, not pure self-play. This is a pragmatic admission that supervised learning is more data-efficient when good training data exists.

**Sources:** [Leela Chess Zero - Wikipedia](https://en.wikipedia.org/wiki/Leela_Chess_Zero), [lczero.org](https://lczero.org/), [GitHub - LeelaChessZero/lc0](https://github.com/LeelaChessZero/lc0), [Lc0 Neural Network Topology](https://lczero.org/dev/backend/nn/)

### 4.2 Other Open-Source AlphaZero-Style Projects

| Project | Language/Framework | Notes |
|---------|--------------------|-------|
| [chess-alpha-zero](https://github.com/Zeta36/chess-alpha-zero) | Python/Keras/TensorFlow | Popular reimplementation; includes supervised pre-training pipeline; reached ~1200 Elo with supervised learning on 10k games |
| [AlphaZero_Chess](https://github.com/geochri/AlphaZero_Chess) | Python/PyTorch | Full pipeline: MCTS self-play -> training -> evaluation; 19 residual blocks, 256 filters; single CUDA GPU + 6 CPU workers |
| [chess-self-play](https://github.com/saurabhk7/chess-self-play) | Python/PyTorch+TensorFlow | Simplified AlphaGo Zero implementation; designed to be adaptable to any two-player game |
| [lunachess](https://github.com/lipeeeee/lunachess) | Python/PyTorch | Deep RL chess engine; trains through pure self-play |
| [Chess-RL](https://github.com/raphcwj/Chess-RL) | Python | Offline RL with DDQN-MCTS approach |

### 4.3 Frameworks and Libraries

| Library | Purpose | Link |
|---------|---------|------|
| **python-chess** | Board representation, legal move generation, PGN parsing, FEN support, UCI engine communication | [python-chess.readthedocs.io](https://python-chess.readthedocs.io/) |
| **PyTorch** | Neural network training (preferred for research flexibility) | [pytorch.org](https://pytorch.org/) |
| **OpenSpiel** | DeepMind's framework for RL in games; includes chess, AlphaZero implementation, and many RL algorithms | [github.com/google-deepmind/open_spiel](https://github.com/google-deepmind/open_spiel) |
| **NNUE-PyTorch** | Stockfish's official NNUE training framework | [github.com/official-stockfish/nnue-pytorch](https://github.com/official-stockfish/nnue-pytorch) |

### 4.4 Essential Reference: "Neural Networks for Chess" (Free Book)

Dominik Klein's book "Neural Networks for Chess" (2022, available free on [arXiv](https://arxiv.org/abs/2209.01506) and [GitHub](https://github.com/asdfjkl/neural_network_chess)) is the single best resource for this project. It covers:
- Chapter 2: Neural network fundamentals (perceptron, backprop, CNNs, batch norm)
- Chapter 3: Classical search techniques for chess
- Chapter 4: How AlphaZero, Leela Chess Zero, and Stockfish NNUE work
- Chapter 5: **Implementing a miniaturized AlphaZero with self-play RL** (the most directly relevant chapter)

**Sources:** [arXiv - Neural Networks for Chess](https://arxiv.org/abs/2209.01506), [GitHub - neural_network_chess](https://github.com/asdfjkl/neural_network_chess)

---

## 5. Practical Considerations for a Personal Project

### 5.1 Compute Requirements: AlphaZero vs Reality

| Configuration | Hardware | Training Time | Expected Strength |
|--------------|----------|---------------|-------------------|
| **AlphaZero (original)** | 5,000 TPUs (self-play) + 64 TPUs (training) | 9 hours | Superhuman (~3500+ Elo) |
| **Leela Chess Zero** | Hundreds of volunteer GPUs over years | Years (cumulative) | Superhuman (~3400+ Elo) |
| **Single high-end GPU (pure self-play)** | 1x RTX 3090/4090 | Weeks to months | ~1000-1500 Elo (estimate) |
| **Single GPU (supervised + self-play)** | 1x RTX 3090/4090 | Days to weeks | ~1200-1800 Elo |
| **Budget GPU (supervised only)** | 1x GTX 1060 or similar | Days | ~800-1200 Elo |

### 5.2 Why Pure Self-Play Is So Slow on Consumer Hardware

AlphaZero's 9-hour training time is misleading for personal projects. Those 9 hours used 5,000 TPUs generating self-play data in parallel. The **effective compute** was:

- 5,000 TPUs x 9 hours = ~45,000 TPU-hours of self-play generation
- 64 TPUs x 9 hours = ~576 TPU-hours of training

On a single consumer GPU (roughly ~1 TPU equivalent for inference), generating the same volume of self-play data would take approximately **5+ years of continuous computation**. This is why pure self-play replication on consumer hardware is impractical without algorithmic shortcuts.

### 5.3 Practical Approaches for Consumer Hardware

**Approach 1: Supervised Pre-Training + Self-Play Fine-Tuning (Recommended)**

1. Download a large database of human games (e.g., Lichess database, freely available, millions of games)
2. Train the neural network to predict human moves (supervised learning on policy) and game outcomes (supervised learning on value)
3. This gets you to a reasonable baseline (~1200-1500 Elo) in days
4. Then switch to self-play RL to improve beyond what human data can teach

This is similar to what the original AlphaGo (pre-Zero) did, and is how several open-source implementations achieve usable strength.

**Approach 2: Smaller Network + Fewer Simulations**

- Use 5-10 residual blocks instead of 19 (AlphaZero's)
- Use 64-128 filters instead of 256
- Use 100-400 MCTS simulations per move instead of 800
- Accept weaker play in exchange for faster training iterations

A network with 7 residual blocks and 256 filters trained on ~10,000 supervised games reached ~1200 Elo with 1,200 MCTS simulations per move.

**Approach 3: Contribute to Leela Chess Zero**

Rather than training from scratch, run the lc0 client to contribute to the distributed training effort and study how the existing networks play. Use pre-trained lc0 networks in your own engine.

**Approach 4: Use Search-Contempt (Cutting-Edge, 2025)**

A recent paper (April 2025) introduces "search-contempt," a hybrid MCTS algorithm that fundamentally alters the distribution of training positions to prefer more challenging ones. This reportedly reduces the required training games from tens of millions to hundreds of thousands, potentially making consumer-GPU self-play training from zero feasible for the first time.

**Sources:** [Budget AlphaZero - Medium](https://hengbin.medium.com/training-budget-alphazero-to-play-chess-with-an-8-month-gpu-in-pytorch-d8e3d2556c16), [Search-Contempt Paper](https://arxiv.org/abs/2504.07757), [chess-alpha-zero](https://github.com/Zeta36/chess-alpha-zero), [Leela Chess Zero - Wikipedia](https://en.wikipedia.org/wiki/Leela_Chess_Zero)

### 5.4 How Many Self-Play Games to Reach Various Skill Levels

This is one of the hardest questions to answer precisely because it depends heavily on network size, MCTS simulation count, and learning efficiency. The following are rough estimates based on community implementations:

| Target Elo | Approach | Estimated Games | Estimated Time (1 GPU) |
|------------|----------|-----------------|------------------------|
| ~800 (beginner) | Pure self-play, small network | 10,000-50,000 | Days |
| ~1200 (intermediate) | Supervised on human games | 10,000 supervised games | Days |
| ~1500 (club player) | Supervised + self-play | 50,000-200,000 self-play + supervised base | 1-4 weeks |
| ~2000 (strong club) | Extended self-play, larger network | 500,000+ | 1-3 months |
| ~2500+ (master) | Massive self-play or distributed | Millions | Not feasible on single GPU |

[Confidence: Medium -- these are extrapolations from limited data points, not rigorous benchmarks.]

### 5.5 Playing Against Humans vs Self-Play for Learning

**Self-play advantages:**
- Unlimited data generation (no dependency on opponents)
- Both sides learn simultaneously
- Games can be generated much faster than real-time
- No latency or scheduling issues

**Playing against humans (or other engines):**
- KnightCap improved from 1650 to 2150 Elo in 308 games on internet chess servers using TD-Leaf learning
- Exposure to diverse strategies (self-play can develop a narrow style)
- More realistic training signal

**Verdict:** Self-play is the standard approach for training. However, periodically testing against external engines (Stockfish at various levels) is valuable for measuring progress and adding diversity to the training distribution.

**Sources:** [Baxter et al. - KnightCap](https://link.springer.com/article/10.1023/A:1007634325138), [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404)

---

## 6. Alternative Approaches

### 6.1 TD-Learning with Traditional Search (Simplest RL Approach)

Instead of MCTS + deep network, use a shallow neural network (or even linear features) as the evaluation function in a traditional alpha-beta search engine, and train it via temporal difference learning.

**How it works:**
1. Start with a random evaluation function
2. Play games (self-play or against opponents)
3. After each move, update the evaluation function using TD(lambda) to reduce the difference between the current position's evaluation and the next position's evaluation
4. The value propagates backward from game outcomes through intermediate positions

**Historical results:**
- **KnightCap:** 1650 -> 2150 Elo in 3 days, 308 games (using hand-crafted features + TD-Leaf)
- **Giraffe** (2015): Reached FIDE International Master level (~2400 Elo) using deep RL with TD-Leaf on a single machine, with a training time of approximately 72 hours

**Advantages:** Much simpler to implement; works with existing alpha-beta search code; no MCTS needed.
**Disadvantages:** Typically requires some hand-crafted features; may plateau at lower strength than AlphaZero-style approaches.

**Sources:** [Chessprogramming Wiki - TD Learning](https://www.chessprogramming.org/Temporal_Difference_Learning), [Giraffe - arXiv](https://arxiv.org/abs/1509.01549), [MIT Technology Review - Giraffe](https://www.technologyreview.com/2015/09/14/247956/deep-learning-machine-teaches-itself-chess-in-72-hours-plays-at-international-master/)

### 6.2 NNUE-Style Shallow Networks (Hybrid Approach)

NNUE (Efficiently Updatable Neural Network) is the approach used by modern Stockfish. It combines:
- **Hand-crafted input features** (piece-square tables with king-relative indexing)
- **Shallow neural network** (3-4 layers, a few million weights)
- **Traditional alpha-beta search** (not MCTS)
- **CPU-optimized** inference (no GPU needed)

**Why it matters for a personal project:** NNUE is vastly simpler to train and run than an AlphaZero-style system. You can train an NNUE network using Stockfish-evaluated positions as targets (supervised learning), and it runs fast enough on CPU for real-time play.

**Training approach:**
1. Generate millions of positions by playing engine games
2. Evaluate each position with Stockfish (or another strong engine)
3. Train the NNUE network to predict Stockfish's evaluations
4. Integrate the trained network into an alpha-beta search engine

**Advantages:** Runs on CPU; extremely fast inference; proven architecture; Stockfish's NNUE training tools are open source.
**Disadvantages:** Not "self-learning from zero" in the purest sense; relies on existing engine evaluations for training data. However, you could use self-play game outcomes as training targets instead.

**Sources:** [Chessprogramming Wiki - NNUE](https://www.chessprogramming.org/NNUE), [Stockfish NNUE](https://www.chessprogramming.org/Stockfish_NNUE), [Wikipedia - NNUE](https://en.wikipedia.org/wiki/Efficiently_updatable_neural_network), [Stockfish Blog](https://stockfishchess.org/blog/2020/introducing-nnue-evaluation/)

### 6.3 Genetic/Evolutionary Approaches

Evolutionary strategies can train neural networks for game play by:
1. Maintaining a population of neural networks (each with different weights)
2. Having them play against each other (tournament selection)
3. Reproducing the best performers with mutations
4. Repeating

**Research findings:**
- Genetic algorithms are "a competitive alternative for training deep neural networks for reinforcement learning" (Uber AI Labs, 2017)
- Evolutionary processes have produced "different grandmaster-level programs" in chess
- OpenAI's evolution strategies work as "a scalable alternative to reinforcement learning"

**Advantages:** No gradient computation needed; highly parallelizable; naturally avoids local minima.
**Disadvantages:** Very data-inefficient compared to gradient-based methods; requires large populations; convergence can be slow for complex problems like chess.

**Verdict:** Evolutionary approaches are interesting but generally less efficient than gradient-based RL for chess. They may be useful as a secondary diversity mechanism rather than the primary training method.

**Sources:** [Uber AI Labs - Deep Neuroevolution](https://arxiv.org/abs/1712.06567), [OpenAI - Evolution Strategies](https://openai.com/index/evolution-strategies/), [Genetic Algorithms for Chess](https://arxiv.org/pdf/1711.08337), [Chessprogramming Wiki - Genetic Programming](https://www.chessprogramming.org/Genetic_Programming)

### 6.4 Q-Learning / DQN Approaches

Direct Q-learning (learning to estimate the value of state-action pairs) has been attempted for chess but faces challenges:
- The action space is very large (~20-30 legal moves per position, from a total of ~4,672 possible move encodings)
- Offline-to-online transition causes instability and catastrophic forgetting
- Actor-critic methods are generally preferred over pure value-based methods for chess

One implementation (DDQN-MCTS) combines Deep Double Q-Networks with MCTS, showing that hybrid approaches can work. However, these tend to be less effective than the AlphaZero policy+value approach.

**Sources:** [Chess-RL GitHub](https://github.com/raphcwj/Chess-RL), [Knightmare Protocol - Why Supervised Q-Learning Broke](https://knightmareprotocol.hashnode.dev/we-had-a-good-run-dueling-ddqn-and-i), [Chessprogramming Wiki - RL](https://www.chessprogramming.org/Reinforcement_Learning)

### 6.5 Feature-Based Learning vs Raw Board State

| Approach | Input | Advantages | Disadvantages |
|----------|-------|------------|---------------|
| **Raw board state** (AlphaZero-style) | Piece positions as multi-plane tensor | No hand-crafted features; network discovers all patterns | Requires much more data and compute |
| **Hand-crafted features** (traditional) | Material count, pawn structure, king safety, mobility, etc. | Much faster to learn; human-interpretable | Limited by feature designer's knowledge; misses patterns human did not encode |
| **Hybrid (NNUE-style)** | Piece-square features + small NN | Fast inference; good performance; trainable | Some feature engineering needed |

**Practical recommendation:** For a personal project, starting with some hand-crafted features (material, piece-square tables, basic king safety) and letting the network learn refinements is far more compute-efficient than starting from raw board state.

---

## 7. Technical Architecture Details

### 7.1 Board Representation for Neural Networks

**AlphaZero's approach (8x8x119 tensor):**
- 8 time steps (current position + 7 move history)
- Per time step: 12 planes (6 piece types x 2 colors, binary: piece present or not)
- 8 x 12 = 96 planes for piece positions over time
- 23 auxiliary planes: castling rights (4), side to move (1), total move count (1), no-progress count (1), repetition indicators
- Board is always oriented from the perspective of the player to move

**Simpler alternatives for a personal project:**
- **Minimal:** 12 planes (6 piece types x 2 colors) for current position only = 8x8x12
- **With history:** Add 1-3 previous positions = 8x8x(12 * N)
- **One-hot per square:** 13 values per square (6 piece types x 2 colors + empty) = 8x8x13

**Sources:** [Chessprogramming Wiki - AlphaZero](https://www.chessprogramming.org/AlphaZero), [Stanford CS231n - ConvChess](https://cs231n.stanford.edu/reports/2015/pdfs/ConvChess.pdf), [ResearchGate - Board Tensor](https://www.researchgate.net/figure/The-representation-of-an-initial-chess-board-as-8-8-12-Tensor_fig3_321028267)

### 7.2 Move Encoding Schemes

**AlphaZero's scheme (4,672 outputs):**
- 64 source squares x 73 move types
- 73 move types = 56 "queen moves" (7 distances x 8 directions) + 8 knight moves + 9 underpromotions (3 piece types x 3 directions)
- The move is encoded by source square + move type
- Illegal moves are masked to zero probability

**Leela Chess Zero's scheme (1,858 outputs):**
- Maps to 80x8x8 = 5,120 spatial outputs, filtered to 1,858 unique legal move slots
- More compact than AlphaZero's encoding

**Simplified scheme for personal projects:**
- Output from-square (64) and to-square (64) separately, plus promotion piece type
- Or enumerate all possible legal moves in a fixed order (~1,792 possible moves if considering only structurally possible moves from any position)

**Sources:** [Chessprogramming Wiki - AlphaZero](https://www.chessprogramming.org/AlphaZero), [Lc0 Neural Network Topology](https://lczero.org/dev/backend/nn/), [Quora - Chess NN Encoding](https://www.quora.com/If-you-were-training-a-neural-network-to-play-chess-how-do-you-encode-the-inputs-board-and-outputs-move-of-the-network)

### 7.3 Training Loop Design

A complete training system has three concurrent components:

```
+-------------------+     game data     +-------------------+
|   SELF-PLAY       | ----------------> |   TRAINING        |
|   WORKERS         |                   |   PROCESS         |
|                   |     new weights   |                   |
|   (CPU + GPU      | <---------------- |   (GPU-intensive) |
|    inference)     |                   |                   |
+-------------------+                   +-------------------+
                                               |
                                               v
                                        +-------------------+
                                        |   EVALUATOR       |
                                        |   (plays new vs   |
                                        |    old model)     |
                                        +-------------------+
```

**Self-play worker:**
1. Load current neural network weights
2. For each game:
   a. Initialize board
   b. For each move: run MCTS (N simulations), select move based on visit counts + temperature
   c. Store (position, MCTS_policy, _) for each move
   d. When game ends, fill in outcome: (position, MCTS_policy, result)
3. Add all game data to a replay buffer
4. Repeat

**Training process:**
1. Sample random mini-batches from the replay buffer
2. Loss = policy_loss + value_loss + L2_regularization
   - policy_loss = cross-entropy between network policy output and MCTS visit distribution
   - value_loss = MSE between network value output and game outcome
3. Update weights via SGD or Adam optimizer
4. Periodically save checkpoint

**Evaluator (optional but recommended):**
1. Play N games between the new model and the previous best model
2. If the new model wins more than 55% of games, adopt it as the new best
3. This prevents training regressions

**Sources:** [AlphaZero_Chess GitHub](https://github.com/geochri/AlphaZero_Chess), [chess-alpha-zero GitHub](https://github.com/Zeta36/chess-alpha-zero), [Medium - How to Build Your Own AlphaZero](https://medium.com/applied-data-science/how-to-build-your-own-alphazero-ai-using-python-and-keras-7f664945c188)

### 7.4 Evaluating Improvement Over Time

**Method 1: Play against benchmark engines**
- Play 100+ games against Stockfish at fixed depth/skill levels
- Compute win/draw/loss percentages
- Estimate Elo from win rate: `Elo_diff = -400 * log10(1/score - 1)` where score = (wins + 0.5*draws) / total

**Method 2: Self-play Elo tracking**
- Maintain a pool of past model checkpoints
- Play new model against past versions
- Track Elo progression using standard rating calculations
- AlphaZero used this approach internally

**Method 3: Average Centipawn Loss (ACPL)**
- Analyze the model's games with a strong engine (Stockfish)
- Measure average centipawn loss per move
- Approximate Elo: `Elo ~= 3100 * e^(-0.01 * ACPL)`

**Method 4: Puzzle/Tactic accuracy**
- Test the model on standardized tactical puzzles
- Measure solve rate as a function of puzzle difficulty
- Provides a different dimension of evaluation than game play

**Sources:** [Elo Rating System - Wikipedia](https://en.wikipedia.org/wiki/Elo_rating_system), [Lichess Forum - ACPL and Elo](https://lichess.org/forum/general-chess-discussion/how-to-estimate-your-elo-for-a-game-using-acpl-and-what-it-realistically-means), [ResearchGate - AlphaZero Training Elo](https://www.researchgate.net/figure/Training-AlphaZero-for-700-000-steps-Elo-ratings-were-computed-from-evaluation-games_fig1_321571298)

---

## 8. Key Challenges and Pitfalls

### 8.1 The Massive State Space

Chess has approximately 10^44 legal positions and an average game tree complexity of 10^123. This means:
- The neural network cannot memorize positions; it must generalize
- MCTS can only explore a tiny fraction of the tree per move
- The quality of the neural network's guidance (policy) is critical for efficient search

**Mitigation:** This is fundamentally why MCTS + neural network works -- the network learns patterns that generalize across positions, and MCTS provides a mechanism to verify and refine the network's suggestions through lookahead.

### 8.2 Training Instability

Self-play training can be unstable because:
- The training data distribution shifts as the model improves (non-stationary)
- Catastrophic forgetting: the model may forget how to handle positions it saw earlier
- The model plays both sides, so a bad update affects both the "player" and the "opponent"

**Mitigations:**
- Use a **replay buffer** with positions from many past iterations, not just the latest
- **Checkpoint evaluation**: only adopt new weights if they demonstrably outperform the previous version
- **Learning rate scheduling**: reduce learning rate as training progresses
- **L2 regularization**: prevents weights from growing too large

**Sources:** [Medium - Why Supervised Q-Learning Broke](https://knightmareprotocol.hashnode.dev/we-had-a-good-run-dueling-ddqn-and-i), [Medium - RL in Chess](https://medium.com/@samgill1256/reinforcement-learning-in-chess-73d97fad96b3)

### 8.3 Catastrophic Forgetting

Specific to chess RL, the model may:
- Learn to play well in certain types of positions but forget others
- Develop a narrow opening repertoire (only experiencing positions it generates)
- Lose previously-learned endgame knowledge when trained on midgame positions

**Mitigations:**
- Diverse opening positions (start self-play from random positions, not always the starting position)
- Replay buffer that retains old data
- Periodic evaluation against a fixed test set of positions across all game phases

### 8.4 Exploration vs Exploitation

**The dilemma:** The model must balance:
- **Exploitation:** Playing the best known moves to generate high-quality training data
- **Exploration:** Trying new moves to discover better strategies

**In MCTS:** The PUCT formula handles this -- c_puct controls the exploration/exploitation balance. Higher c_puct = more exploration.

**In move selection during training:** Temperature controls randomness:
- High temperature = more random moves (exploration)
- Low temperature = greedily choose the most-visited move (exploitation)
- AlphaZero uses temperature=1 for the first 30 moves, then drops to near-zero

**In training data:** Dirichlet noise is added to the root node's prior probabilities to ensure the search occasionally explores unlikely moves.

### 8.5 How Long Before "Reasonable" Chess

Based on available evidence:
- **Random play (Elo ~0):** Starting point -- moves are legal but nonsensical
- **After ~1,000 games (small network):** The model begins to capture pieces and avoid obvious blunders (~400-600 Elo)
- **After ~10,000 games:** Basic tactical awareness, simple plans (~800-1000 Elo)
- **After ~100,000 games:** "Reasonable" chess -- coherent openings, tactical competence (~1200-1500 Elo)
- **After ~1,000,000 games:** Strong amateur play (~1800-2200 Elo)

These numbers assume a network of moderate size (5-10 residual blocks) with 200-800 MCTS simulations per move. Larger networks and more simulations improve quality per game but take longer to compute.

[Confidence: Medium -- extrapolated from multiple sources with varying conditions]

**Sources:** [Silver et al., Science 2018](https://www.science.org/doi/10.1126/science.aar6404), [chess-alpha-zero GitHub](https://github.com/Zeta36/chess-alpha-zero), [Search-Contempt Paper](https://arxiv.org/abs/2504.07757)

---

## 9. Recommended Approach for a Personal Project

### 9.1 Decision Framework

Choose your approach based on your primary goal:

| Goal | Recommended Approach | Estimated Time | Hardware Needed |
|------|---------------------|----------------|-----------------|
| **Learn about RL/AI** | Build mini-AlphaZero from scratch, start with simpler games (Connect4, then chess) | 2-4 weeks | Any GPU or CPU |
| **Build a chess engine that learns** | TD-learning + alpha-beta search | 1-2 weeks to implement, days to train | CPU only |
| **Reach strong amateur play (1500+)** | Supervised pre-training + self-play fine-tuning | 2-4 weeks | Consumer GPU (RTX 3060+) |
| **Replicate AlphaZero at scale** | Contribute to Leela Chess Zero or use cloud compute | Ongoing | GPU farm or cloud budget |
| **Minimize code, maximize learning** | Use OpenSpiel framework with built-in AlphaZero | 1-2 weeks | Consumer GPU |

### 9.2 Recommended Path: Phased Approach

**Phase 1: Foundation (Week 1-2)**
1. Set up the environment: Python, PyTorch, python-chess
2. Implement board representation (8x8x12 minimum, expand later)
3. Implement a simple move encoding scheme
4. Build a basic neural network with 3-5 residual blocks, 64-128 filters
5. Implement MCTS with PUCT selection
6. Test on a simpler game first (Connect4 or Tic-Tac-Toe) to validate the pipeline

**Phase 2: Self-Play Pipeline (Week 2-3)**
1. Implement the self-play loop (game generation -> data collection -> training)
2. Start with 100-200 MCTS simulations per move (fast iteration)
3. Implement a replay buffer
4. Add temperature-based exploration
5. Run initial self-play training: the agent should learn to capture pieces and avoid mate within a few hundred games

**Phase 3: Supervised Bootstrapping (Week 3-4)**
1. Download Lichess game database (free, millions of games)
2. Train the network on human move predictions (supervised policy) and game outcomes (supervised value)
3. This should reach ~1000-1200 Elo relatively quickly
4. Use this as the starting point for self-play refinement

**Phase 4: Self-Play Refinement (Week 4+)**
1. Switch to self-play training using the supervised pre-trained network
2. Increase MCTS simulations to 400-800 per move
3. Track Elo progression by playing against Stockfish at various levels
4. Implement evaluation matches between old and new checkpoints
5. Experiment with network size, learning rate, and replay buffer management

**Phase 5: Optimization (Ongoing)**
1. Profile and optimize MCTS (the bottleneck is usually neural network inference during search)
2. Consider implementing the network in C++ for faster inference
3. Experiment with larger networks if compute allows
4. Add opening book randomization for self-play diversity

### 9.3 Minimal Viable Architecture

```
Input: 8x8x17 tensor
  - 12 planes for pieces (6 types x 2 colors)
  - 1 plane for side to move
  - 4 planes for castling rights

Network:
  - 1 conv layer (3x3, 128 filters) + batch norm + ReLU
  - 5 residual blocks (each: 2 x [3x3 conv, 128 filters, batch norm, ReLU] + skip)
  - Policy head: 1x1 conv -> FC -> softmax over 1858 moves (lc0 scheme)
  - Value head: 1x1 conv -> 128 FC -> ReLU -> 1 FC -> tanh

MCTS:
  - 200 simulations per move (training)
  - 800 simulations per move (evaluation/play)
  - c_puct = 1.0
  - Temperature = 1.0 for first 30 moves, 0.1 thereafter
  - Dirichlet noise: alpha=0.3, epsilon=0.25 at root

Training:
  - Batch size: 256-512
  - Learning rate: 0.01, decay by 0.1 at milestones
  - Optimizer: SGD with momentum 0.9
  - L2 regularization: 1e-4
  - Replay buffer: 500,000 most recent positions
```

### 9.4 Key Libraries to Install

```bash
pip install python-chess   # Board, moves, legal move generation, PGN, FEN, UCI
pip install torch          # Neural network (PyTorch)
pip install numpy          # Numerical operations
pip install tqdm           # Progress bars for training
```

Optional but recommended:
```bash
pip install tensorboard    # Training visualization
pip install stockfish      # Python wrapper for Stockfish engine (for evaluation)
```

---

## 10. Knowledge Gaps

The following areas were searched but yielded insufficient or conflicting evidence:

### 10.1 Precise Elo-to-Games Mapping on Consumer Hardware
**Searched for:** Rigorous benchmarks mapping number of self-play training games to Elo strength on specific consumer GPU configurations.
**Found:** Only anecdotal reports and extrapolations. No standardized benchmark exists.
**Impact:** The training time estimates in Section 5.4 are approximations. Actual results will vary significantly based on network size, MCTS simulation count, and hardware.

### 10.2 Search-Contempt Real-World Results
**Searched for:** Practical implementations and consumer-hardware results using the search-contempt technique from the April 2025 paper.
**Found:** Only the paper itself; no community implementations or replications found yet.
**Impact:** The claim that search-contempt makes consumer-GPU training from zero feasible is promising but unverified by independent replication.

### 10.3 Optimal Hyperparameters for Small-Scale Training
**Searched for:** Tuned hyperparameters (learning rate schedule, MCTS simulations, replay buffer size, temperature schedule) specifically optimized for single-GPU chess training.
**Found:** Most implementations copy AlphaZero's hyperparameters, which were optimized for massive-scale training. Small-scale-specific tuning guidance is scarce.
**Impact:** You may need to experiment significantly with hyperparameters; AlphaZero's defaults may not be optimal for your compute budget.

### 10.4 Transformer Architectures for Small-Scale Chess
**Searched for:** Whether Leela Chess Zero's switch to transformer-based architecture in 2022 yields benefits at small network sizes relevant to personal projects.
**Found:** Lc0's transformer work focuses on large models; no evidence of benefits at small scale.
**Impact:** Residual CNN architecture remains the safe choice for personal projects.

---

## 11. Source Analysis

| Source | Type | Reputation | Independence | Used For |
|--------|------|------------|-------------|----------|
| Silver et al., Science 2018 | Peer-reviewed paper | Tier 1 | Primary source | AlphaZero architecture, training, results |
| DeepMind Blog | Official blog | Tier 1 | Same org as paper | Supplementary details |
| Wikipedia - AlphaZero | Encyclopedia | Tier 2 | Aggregates primary sources | Overview, verification |
| Chessprogramming Wiki | Domain wiki | Tier 2 | Community-maintained | Technical details, historical context |
| Leela Chess Zero (lczero.org) | Official project site | Tier 1 | Independent replication | Open-source implementation details |
| arXiv - Neural Networks for Chess | Pre-print/Book | Tier 2 | Independent author | Comprehensive reference |
| arXiv - Giraffe (Lai, 2015) | Thesis/pre-print | Tier 2 | Independent researcher | TD-learning approach |
| arXiv - Search-Contempt (2025) | Pre-print | Tier 2 | Independent researchers | Efficient training methods |
| arXiv - Deep Neuroevolution | Pre-print | Tier 2 | Uber AI Labs | Evolutionary alternatives |
| PNAS - Chess Knowledge in AlphaZero | Peer-reviewed | Tier 1 | Partially independent | Learned knowledge analysis |
| GitHub implementations | Code repositories | Tier 3 | Various independent devs | Practical implementation details |
| Medium articles | Blog posts | Tier 3 | Various independent devs | Practical experience reports |
| Chessprogramming - NNUE | Domain wiki | Tier 2 | Community-maintained | NNUE architecture details |
| Stockfish blog | Official project | Tier 1 | Independent project | NNUE integration |
| OpenSpiel / DeepMind | Official framework | Tier 1 | DeepMind | Framework option |

---

## 12. References

### Primary Research Papers
1. Silver, D., et al. "A general reinforcement learning algorithm that masters chess, shogi, and Go through self-play." *Science* 362.6419 (2018). [Link](https://www.science.org/doi/10.1126/science.aar6404)
2. Silver, D., et al. "Mastering Chess and Shogi by Self-Play with a General Reinforcement Learning Algorithm." *arXiv preprint* 1712.01815 (2017). [Link](https://arxiv.org/pdf/1712.01815)
3. Lai, M. "Giraffe: Using Deep Reinforcement Learning to Play Chess." *arXiv preprint* 1509.01549 (2015). [Link](https://arxiv.org/abs/1509.01549)
4. McGrath, T., et al. "Acquisition of chess knowledge in AlphaZero." *PNAS* 119.47 (2022). [Link](https://www.pnas.org/doi/10.1073/pnas.2206625119)

### Books and Comprehensive Guides
5. Klein, D. "Neural Networks for Chess." *arXiv preprint* 2209.01506 (2022). [Link](https://arxiv.org/abs/2209.01506) / [GitHub](https://github.com/asdfjkl/neural_network_chess)

### Technical References
6. [AlphaZero - Chessprogramming Wiki](https://www.chessprogramming.org/AlphaZero)
7. [Temporal Difference Learning - Chessprogramming Wiki](https://www.chessprogramming.org/Temporal_Difference_Learning)
8. [NNUE - Chessprogramming Wiki](https://www.chessprogramming.org/NNUE)
9. [Stockfish NNUE - Chessprogramming Wiki](https://www.chessprogramming.org/Stockfish_NNUE)
10. [Reinforcement Learning - Chessprogramming Wiki](https://www.chessprogramming.org/Reinforcement_Learning)

### Open-Source Implementations
11. [Leela Chess Zero](https://lczero.org/) / [GitHub](https://github.com/LeelaChessZero/lc0)
12. [chess-alpha-zero](https://github.com/Zeta36/chess-alpha-zero)
13. [AlphaZero_Chess (PyTorch)](https://github.com/geochri/AlphaZero_Chess)
14. [chess-self-play](https://github.com/saurabhk7/chess-self-play)
15. [OpenSpiel](https://github.com/google-deepmind/open_spiel)
16. [python-chess](https://python-chess.readthedocs.io/)
17. [NNUE-PyTorch](https://github.com/official-stockfish/nnue-pytorch)

### Efficient Training Research
18. "Search-contempt: a hybrid MCTS algorithm for training AlphaZero-like engines with better computational efficiency." *arXiv preprint* 2504.07757 (2025). [Link](https://arxiv.org/abs/2504.07757)

### Historical RL in Games
19. Tesauro, G. "Temporal Difference Learning and TD-Gammon." [Link](https://www.bkgm.com/articles/tesauro/tdl.html)
20. Baxter, J., Tridgell, A., Weaver, L. "Learning to Play Chess Using Temporal Differences." *Machine Learning* 40 (2000). [Link](https://link.springer.com/article/10.1023/A:1007634325138)

### Evolutionary Approaches
21. Such, F.P., et al. "Deep Neuroevolution: Genetic Algorithms Are a Competitive Alternative for Training Deep Neural Networks for Reinforcement Learning." *arXiv preprint* 1712.06567 (2017). [Link](https://arxiv.org/abs/1712.06567)
22. Salimans, T., et al. "Evolution Strategies as a Scalable Alternative to Reinforcement Learning." OpenAI (2017). [Link](https://openai.com/index/evolution-strategies/)

### Supplementary Sources
23. [AlphaZero - Wikipedia](https://en.wikipedia.org/wiki/AlphaZero)
24. [Leela Chess Zero - Wikipedia](https://en.wikipedia.org/wiki/Leela_Chess_Zero)
25. [NNUE - Wikipedia](https://en.wikipedia.org/wiki/Efficiently_updatable_neural_network)
26. [Lc0 Neural Network Topology](https://lczero.org/dev/backend/nn/)
27. [Introducing NNUE Evaluation - Stockfish](https://stockfishchess.org/blog/2020/introducing-nnue-evaluation/)
28. [DeepMind Blog - AlphaZero](https://deepmind.google/blog/alphazero-shedding-new-light-on-chess-shogi-and-go/)
