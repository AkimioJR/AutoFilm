#!/usr/bin/env python3

import sys
import asyncio
from sys import path
from os.path import dirname

# 添加项目根目录到路径
path.append(dirname(dirname(__file__)))

from app.core.config import settings

# 延迟导入，避免sklearn依赖问题
# 只在需要时才导入相关模块
def get_alist2strm():
    try:
        from app.modules.alist2strm import Alist2Strm
        return Alist2Strm
    except ImportError as e:
        print(f"导入Alist2Strm模块时出错: {e}")
        print("请确保已安装所有依赖库，或者检查Docker环境中是否缺少必要的系统库（如libgomp.so.1）")
        sys.exit(1)


def get_ani2alist():
    try:
        from app.modules.ani2alist import Ani2Alist
        return Ani2Alist
    except ImportError as e:
        print(f"导入Ani2Alist模块时出错: {e}")
        print("请确保已安装所有依赖库，或者检查Docker环境中是否缺少必要的系统库（如libgomp.so.1）")
        sys.exit(1)


def get_libraryposter():
    try:
        from app.modules.libraryposter import LibraryPoster
        return LibraryPoster
    except ImportError as e:
        print(f"导入LibraryPoster模块时出错: {e}")
        print("请确保已安装所有依赖库，或者检查Docker环境中是否缺少必要的系统库（如libgomp.so.1）")
        sys.exit(1)


def find_task_by_id(task_list, task_id):
    """根据ID查找任务配置"""
    for task in task_list:
        if task.get('id') == task_id:
            return task
    return None


def main():
    if len(sys.argv) != 2:
        print("使用方法: python run.py <任务ID>")
        print("可用的任务ID:")
        
        # 显示所有Alist2Strm任务ID
        for task in settings.AlistServerList:
            print(f"  Alist2Strm: {task.get('id')}")
            
        # 显示所有Ani2Alist任务ID
        for task in settings.Ani2AlistList:
            print(f"  Ani2Alist: {task.get('id')}")
            
        # 显示所有LibraryPoster任务ID
        for task in settings.LibraryPosterList():
            print(f"  LibraryPoster: {task.get('id')}")
            
        return
    
    task_id = sys.argv[1]
    
    # 查找Alist2Strm任务
    alist_task = find_task_by_id(settings.AlistServerList, task_id)
    if alist_task:
        print(f"正在执行 Alist2Strm 任务: {task_id}")
        Alist2Strm = get_alist2strm()
        alist_instance = Alist2Strm(**alist_task)
        asyncio.run(alist_instance.run())
        return
    
    # 查找Ani2Alist任务
    ani_task = find_task_by_id(settings.Ani2AlistList, task_id)
    if ani_task:
        print(f"正在执行 Ani2Alist 任务: {task_id}")
        Ani2Alist = get_ani2alist()
        ani_instance = Ani2Alist(**ani_task)
        asyncio.run(ani_instance.run())
        return
    
    # 查找LibraryPoster任务
    poster_task = find_task_by_id(settings.LibraryPosterList(), task_id)
    if poster_task:
        print(f"正在执行 LibraryPoster 任务: {task_id}")
        LibraryPoster = get_libraryposter()
        poster_instance = LibraryPoster(**poster_task)
        asyncio.run(poster_instance.run())
        return
    
    print(f"未找到ID为 '{task_id}' 的任务")
    print("可用的任务ID:")
    
    # 显示所有Alist2Strm任务ID
    for task in settings.AlistServerList:
        print(f"  Alist2Strm: {task.get('id')}")
        
    # 显示所有Ani2Alist任务ID
    for task in settings.Ani2AlistList:
        print(f"  Ani2Alist: {task.get('id')}")
        
    # 显示所有LibraryPoster任务ID
    for task in settings.LibraryPosterList():
        print(f"  LibraryPoster: {task.get('id')}")


if __name__ == "__main__":
    main()