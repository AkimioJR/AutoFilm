import os
from pathlib import Path
from shutil import rmtree

from setuptools import setup, find_packages  # type: ignore
from Cython.Build import cythonize  # type: ignore


def find_py_files(dir: Path) -> list[str]:
    needed_compiled_files = []
    for file in dir.rglob("*.py"):
        # 添加排除条件
        #if file.name not in ["main.py", "__init__.py", "api.py"]:
        if file.name not in ["main.py"]:
            needed_compiled_files.append(file.as_posix())
    return needed_compiled_files


if __name__ == "__main__":
    files = find_py_files(Path("app"))
    print(f"📝 需要编译的文件: {files}")
    cpu_count = os.cpu_count() or 1
    print(f"🚀 使用 {cpu_count} 个核心编译")
    setup(
        ext_modules=cythonize(
            files,
            compiler_directives={
                "language_level": "3",  # 使用Python 3语法
                "annotation_typing": True,  # 使用类型注解
                "infer_types": True,  # 启用类型推断
            },
            force=True,  # 强制重新编译所有文件
        ),
        packages=find_packages(where="app"),  # 发现app下的所有包
        package_dir={"": "app"},  # 包根目录为app
        script_args=["build_ext", "-j", str(cpu_count), "--inplace"],
    )
    build_dir = Path("build").absolute()
    if build_dir.exists():
        rmtree(build_dir)
        print(f"✅ 已清理 build 目录: {build_dir}")
    else:
        print(f"❌ build 目录不存在: {build_dir}")
