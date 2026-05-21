#!/bin/bash
# 批量修复 evif-fuse/src/lib.rs 中的 async move 借用移动问题

FILE="crates/evif-fuse/src/lib.rs"

# 备份原文件
cp "$FILE" "$FILE.bak"

echo "修复 async move 借用移动问题..."

# 使用 sed 进行替换
sed -i.bak2 '
    # 修复 setattr 函数 (行 330-381)
    /let result = rt.block_on(async move {$/,/});$/{
        /async move {/s//async move { let path_str = path_str.clone();/
        /&path_str)/s//\&path_str)/
    }
' "$FILE.bak"

# 应用另一个修复模式
sed -i.bak3 '
    # 修复 open 函数
    /match rt.block_on(async move {$/,/});$/{
        /async move {/s//async move { let allow_write = allow_write.clone();/
        /&allow_write)/s//\&allow_write)/
    }
' "$FILE.bak2"

# 移动最终文件
mv "$FILE.bak3" "$FILE"

echo "修复完成，备份文件:"
ls -la "$FILE".bak*
