# huff

Rust语言实现的一个基于huffman编码的命令行压缩工具demo.

## 安装

```bash
cargo install --path .

## 使用方法
# Compress
huff compress input.bin -o input.huff

# Decompress
huff decompress input.huff -o output.bin