#!/usr/bin/env python3
"""
系统性修复 evif-fuse/src/lib.rs 编译错误
"""

import re

# 读取文件
with open('crates/evif-fuse/src/lib.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# 修复列表
fixes = []

# 1. 修复 1: 删除重复的 resolve_path 调用
# 检测: 行 135-139 有重复
fixes.append({
    'pattern': r'^(\s+)let evif_path = self\.resolve_path\(1, path\)\?;\n(\s+)debug!\("Getting attributes for: \{\}", evif_path\);\n(\s+)let path_str = path\.to_string_lossy\(\)\.to_string\(\);\n(\s+)let evif_path = self\.resolve_path\(1, path\)\?;\n(\s+)debug!\("Getting attributes for: \{\}", evif_path\);',
    'replacement': r'\1let path_str = path.to_string_lossy().to_string();\n\2let evif_path = self.resolve_path(1, path)?;\n\3debug!("Getting attributes for: {}", evif_path);',
    'description': '删除重复的 resolve_path 调用 - get_attr_async'
})

# 2. 修复 2: readdir_async 中的重复调用
fixes.append({
    'pattern': r'^(\s+)let evif_path = self\.resolve_path\(1, path\)\?;\n(\s+)debug!\("Reading directory: \{\}", evif_path\);\n(\s+)let path_str = path\.to_string_lossy\(\)\.to_string\(\);\n(\s+)let evif_path = self\.resolve_path\(1, path\)\?;\n(\s+)debug!\("Reading directory: \{\}", evif_path\);',
    'replacement': r'\1let path_str = path.to_string_lossy().to_string();\n\2let evif_path = self.resolve_path(1, path)?;\n\3debug!("Reading directory: {}", evif_path);',
    'description': '删除重复的 resolve_path 调用 - readdir_async'
})

# 3. 修复 3: read_async 中的重复调用
fixes.append({
    'pattern': r'^(\s+)let evif_path = self\.resolve_path\(1, path\)\?;\n(\s+)debug!\("Reading file: \{\} \(offset=\{\}, size=\{\}\)", evif_path, offset, size\);\n(\s+)let path_str = path\.to_string_lossy\(\)\.to_string\(\);\n(\s+)let evif_path = self\.resolve_path\(1, path\)\?;\n(\s+)debug!\("Reading file: \{\} \(offset=\{\}, size=\{\}\)", evif_path, offset, size\);',
    'replacement': r'\1let path_str = path.to_string_lossy().to_string();\n\2let evif_path = self.resolve_path(1, path)?;\n\3debug!("Reading file: {} (offset={}, size={})", evif_path, offset, size);',
    'description': '删除重复的 resolve_path 调用 - read_async'
})

# 4. 修复 4: write_async 中的 self.allow_write 访问
fixes.append({
    'pattern': r'^(\s+)if !self\.allow_write \{',
    'replacement': r'\1if !self.allow_write {',
    'description': '修复 write_async 中的 self.allow_write 访问'
})

# 5. 修复 5: create 方法中的 Ok::<(), EvifError>(file_info) 返回类型
fixes.append({
    'pattern': r'Ok::<\(\), EvifError>\(file_info\)',
    'replacement': r'Ok::<FileInfo, EvifError>(file_info)',
    'description': '修复 create 方法返回类型'
})

# 6. 修复 6: mount_evif 函数返回类型
fixes.append({
    'pattern': r'Ok\(\(session\)\n\s*}',
    'replacement': r'Ok(session)\n',
    'description': '修复 mount_evif 返回类型'
})

# 应用修复
content = ''.join(lines)
fix_count = 0

for fix in fixes:
    old_content = content
    content, count = re.subn(fix['pattern'], fix['replacement'], content, flags=re.DOTALL)
    if count > 0:
        print(f"✓ 修复 {fix['description']}: {count} 处")
        fix_count += count

# 写回文件
with open('crates/evif-fuse/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print(f"\n总共修复: {fix_count} 处")
print("修复完成")
