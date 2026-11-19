# layout-analysis

**[日本語](#日本語)** | **[English](#english)**

---

## 日本語

英語のテキストを入れると、文字・二ッ組・三ッ組の出現頻度を調べ、その統計からキーボード配列を評価・最適化するコマンドラインツールです。
日本語ローマ字入力の配列の「絶望度」を測る [ZETSUBOU](https://zetsubou.activetk.jp/) に触発されて作りました。

### できること

| コマンド | 内容 |
|---|---|
| `analyze` | 1文字・二ッ組・三ッ組の出現個数と確率の表 |
| `generate` | 第 k 近似(Shannon)によるランダムな文字列の生成 |
| `eval` | QWERTY・Dvorak・Colemak などの打鍵コストの比較 |
| `optimize` | 焼きなまし法による最適な配列(`--worst` で最悪の「ZETSUBOU 配列」)の探索 |
| `layouts` | 組み込みの配列の一覧 |

### インストール

[Rust](https://www.rust-lang.org/) が必要です。

```bash
cargo install --git https://github.com/ReBot-dev/layout-analysis
```

### 使い方

入力は普通のテキストファイルです。英字は小文字にそろえ、それ以外(数字・記号・改行など)は空白にし、連続した空白は1つにまとめてから数えます。
複数のファイルを渡すとつなげて扱います。

```bash
# 頻度の表(上位 30)
layout-analysis analyze book.txt

# Project Gutenberg の本はライセンス文を除いて読む
layout-analysis analyze --source gutenberg pg11.txt pg1342.txt

# 既存の配列の比較
layout-analysis eval book.txt

# 自分の配列を評価する(30 キーを行の順に。_ は空きキー、/ は読みやすさのための区切り)
layout-analysis eval book.txt -l qwerty -l "qwfpb_luy_/arstgmneio/zxcdvkh___"

# 最適な配列を探す / 最悪の配列を探す
layout-analysis optimize book.txt
layout-analysis optimize book.txt --worst

# 第 3 近似で 100 文字生成
layout-analysis generate book.txt -k 3
```

`--csv` を付けると、`analyze` と `eval` の結果を CSV で出力します。

### 結果の例

英語版 Wikipedia を整形した [text8](http://mattmahoney.net/dc/textdata.html)(1 億文字)での `eval` の結果です。

```
layout         cost E QWERTY=100    SFB%    alt% home dist  keys
qwerty         1.5217     100.0    6.82    52.5     0.683  qwertyuiop/asdfghjkl_/zxcvbnm___
dvorak         0.6648      43.7    3.13    72.0     0.374  ___pyfgcrl/aoeuidhtns/_qjkxbmwvz
colemak        0.6596      43.3    1.18    58.7     0.309  qwfpgjluy_/arstdhneio/zxcvbkm___
colemak-dh     0.6510      42.8    1.18    58.7     0.310  qwfpbjluy_/arstgmneio/zxcdvkh___
workman        0.7922      52.1    3.19    55.4     0.329  qdrwbjfup_/ashtgyneoi/zxmcvkl___
```

同じデータで `optimize` を実行すると(シード 42)、母音が右手、子音が左手に分かれた配列になります。

```
l c m p b | _ u h k q
 r s t n f | y i o e a
  x g d w v | _ _ z j _

cost E        0.4973  (QWERTY = 100: 32.7)
SFB           1.52% of letter bigrams
alternation   73.5% of letter bigrams
```

### 打鍵コストのモデル

QWERTY の文字キーと `; , . /` の位置からなる 3 段 × 10 列の 30 キーに 26 文字を置き(4 キーは空き)、空白は親指のスペースバーに固定します。
上段は -0.25 キー、下段は +0.5 キーずれた一般的な横ずれ配列で、運指は標準的なものです。

1 打鍵あたりの期待コスト E を、確率で重みをつけた次の項の和として計算します。

| 項 | 内容 | 既定値 |
|---|---|---|
| 単打鍵 | ホームポジションからの距離 × 指の重み | 人差指・中指 1.0、薬指 1.2、小指 1.5 |
| 同指連続(SFB) | 同じ指で別のキーを続けて打つ | 5.0 × 重み × 距離 |
| シザース | 同じ手の隣の指で上段と下段 | 2.5 + 0.4 × 距離 |
| 同じ手 | その他の同じ手の連続 | 0.4 × 距離 |
| 左右交互 | 手が入れ替わる | −0.2 |
| 同指跳躍(SFS) | 1 文字おいて同じ指で別のキー | 2.0 × 重み × 距離 |
| 方向転換 | 片手 3 連続で指の進む向きが反転 | 1.0 |

空白を含む二ッ組・三ッ組のコストは数えません。
**これらの係数は打鍵の負担の大小関係を表すために決めた値で、実際の打鍵時間を測って求めたものではありません。**
`--params` で TOML ファイルを渡すと変更できます([`params.example.toml`](params.example.toml))。

数値計算の工夫として、テキストは 1 回だけ走査して確率表にし、キーの組ごとのコストも事前に表にしておきます。
そのため、焼きなまし法の 1 ステップは出現した組(text8 で約 1 万 6 千種類)の積和だけで済み、1 億文字のデータでも `eval` は約 1 秒、`optimize`(30 万ステップ)は数秒で終わります。

### データの入手先(例)

データはこのリポジトリには含まれていません。

| データ | 入手先 | 読み込み方 |
|---|---|---|
| Project Gutenberg の本 | https://www.gutenberg.org/ | `--source gutenberg` |
| Tatoeba の英語文 | https://tatoeba.org/downloads | `--source tatoeba` |
| text8 | http://mattmahoney.net/dc/text8.zip | そのまま |

### 今後の予定

- 日本語ローマ字入力への対応(かな → ローマ字の変換)
- 実際の打鍵データに基づく係数
- 格子配列・分割キーボードなど、ほかのキーの並び

### ライセンス

MIT

---

## English

A command-line tool that counts the letter, bigram and trigram frequencies of English text and uses them to evaluate and optimize keyboard layouts.
Inspired by [ZETSUBOU](https://zetsubou.activetk.jp/), a benchmark of how "hopeless" a layout is for Japanese romaji input.

### Features

| Command | Description |
|---|---|
| `analyze` | Counts and probabilities of letters, bigrams and trigrams |
| `generate` | Random text from the k-th order (Shannon) approximation |
| `eval` | Typing cost of QWERTY, Dvorak, Colemak and others |
| `optimize` | Simulated annealing search for the best layout (or the worst "ZETSUBOU" layout with `--worst`) |
| `layouts` | List of built-in layouts |

### Install

Requires [Rust](https://www.rust-lang.org/).

```bash
cargo install --git https://github.com/ReBot-dev/layout-analysis
```

### Usage

The input is plain text. Letters are lowercased, everything else (digits, symbols, line breaks, ...) becomes a space, and runs of spaces are collapsed before counting.
Several files are concatenated.

```bash
# frequency tables (top 30)
layout-analysis analyze book.txt

# Project Gutenberg books: strip the license header and footer
layout-analysis analyze --source gutenberg pg11.txt pg1342.txt

# compare existing layouts
layout-analysis eval book.txt

# evaluate your own layout (30 keys in row order; _ = empty key, / is an optional row separator)
layout-analysis eval book.txt -l qwerty -l "qwfpb_luy_/arstgmneio/zxcdvkh___"

# search for the best / worst layout
layout-analysis optimize book.txt
layout-analysis optimize book.txt --worst

# 100 characters of the 3rd order approximation
layout-analysis generate book.txt -k 3
```

`--csv` prints the results of `analyze` and `eval` as CSV.

### Example

`eval` on [text8](http://mattmahoney.net/dc/textdata.html) (100 million characters of cleaned English Wikipedia):

```
layout         cost E QWERTY=100    SFB%    alt% home dist  keys
qwerty         1.5217     100.0    6.82    52.5     0.683  qwertyuiop/asdfghjkl_/zxcvbnm___
dvorak         0.6648      43.7    3.13    72.0     0.374  ___pyfgcrl/aoeuidhtns/_qjkxbmwvz
colemak        0.6596      43.3    1.18    58.7     0.309  qwfpgjluy_/arstdhneio/zxcvbkm___
colemak-dh     0.6510      42.8    1.18    58.7     0.310  qwfpbjluy_/arstgmneio/zxcdvkh___
workman        0.7922      52.1    3.19    55.4     0.329  qdrwbjfup_/ashtgyneoi/zxmcvkl___
```

`optimize` on the same data (seed 42) puts the vowels on the right hand and the consonants on the left:

```
l c m p b | _ u h k q
 r s t n f | y i o e a
  x g d w v | _ _ z j _

cost E        0.4973  (QWERTY = 100: 32.7)
SFB           1.52% of letter bigrams
alternation   73.5% of letter bigrams
```

### Cost model

The 26 letters are placed on 30 keys, 3 rows x 10 columns (the QWERTY letter keys plus `; , . /`; 4 keys stay empty). Space is always on the thumb's space bar.
The keyboard is row-staggered (top row -0.25 keys, bottom row +0.5 keys) with standard fingering.

The expected cost per keystroke E is the probability-weighted sum of:

| Term | Meaning | Default |
|---|---|---|
| Single key | distance from the home key x finger weight | index/middle 1.0, ring 1.2, pinky 1.5 |
| Same-finger bigram (SFB) | one finger, two different keys in a row | 5.0 x weight x distance |
| Scissor | adjacent fingers of one hand, top <-> bottom row | 2.5 + 0.4 x distance |
| Same hand | other same-hand pairs | 0.4 x distance |
| Alternation | hands switch | -0.2 |
| Same-finger skipgram (SFS) | 1st and 3rd key on one finger | 2.0 x weight x distance |
| Redirect | one-hand trigram whose finger direction reverses | 1.0 |

Bigrams and trigrams containing a space are not counted.
**The coefficients express the relative effort of each movement; they are not fitted to measured typing times.**
Override them with a TOML file via `--params` (see [`params.example.toml`](params.example.toml)).

For speed, the text is scanned once into probability tables and the cost of every key pair and triple is precomputed.
One annealing step is then a sum over the n-grams that occur (about 16,000 for text8): `eval` on 100 million characters takes about a second and `optimize` (300,000 steps) a few seconds.

### Data sources (examples)

No data is included in this repository.

| Data | Source | How to read |
|---|---|---|
| Project Gutenberg books | https://www.gutenberg.org/ | `--source gutenberg` |
| Tatoeba English sentences | https://tatoeba.org/downloads | `--source tatoeba` |
| text8 | http://mattmahoney.net/dc/text8.zip | as is |

### Roadmap

- Japanese romaji input (kana-to-romaji conversion)
- Coefficients fitted to real keystroke data
- Other key geometries (ortholinear, split)

### License

MIT
