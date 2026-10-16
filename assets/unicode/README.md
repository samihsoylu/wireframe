# Unicode data

Emoji property tables use Unicode 17.0, matching unicode-segmentation 1.13.3.

Sources:
- https://www.unicode.org/Public/17.0.0/ucd/emoji/emoji-data.txt
- https://www.unicode.org/Public/17.0.0/ucd/emoji/emoji-variation-sequences.txt

The derived tables are distributed under the included Unicode License V3.

Source SHA256:
- emoji-data.txt: 2cb2bb9455cda83e8481541ecf5b6dfda66a3bb89efa3fa7c5297eccf607b72b
- emoji-variation-sequences.txt: bb3d09ef03f206012c7532dd52dc0a21c9efddba0135ea4cf0d9201b8b9bba7e
- LICENSE.txt: e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96

Regenerate with `python scripts/build_emoji_data.py /path/to/unicode-sources`. The generator verifies the two source hashes and formats the resulting Rust table.
