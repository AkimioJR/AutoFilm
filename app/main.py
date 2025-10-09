from asyncio import get_event_loop
from sys import path
from os.path import dirname

path.append(dirname(dirname(__file__)))

from apscheduler.schedulers.asyncio import AsyncIOScheduler  # type:ignore
from apscheduler.triggers.cron import CronTrigger  # type:ignore

from app.core import settings, logger
from app.extensions import LOGO
from app.modules import Alist2Strm, Ani2Alist
from fastapi import FastAPI
import uvicorn
from app.modules import Alist2Strm, Ani2Alist, LibraryPoster


def print_logo() -> None:
    """
    打印 Logo
    """

    print(LOGO)
    print(f" {settings.APP_NAME} {settings.APP_VERSION} ".center(65, "="))
    print("")

from app.modules.api import router as api_router

def start_api_server():
    app = FastAPI(title=settings.APP_NAME)
    app.include_router(api_router)

    uvicorn.run(
        app,
        host="0.0.0.0",
        port=settings.API_PORT,
        log_config=None
    )

def __mkdir(self) -> None:
    """
    创建目录
    """
    # 修改前：使用 with self.CONFIG_DIR as dir_path:
    if not self.CONFIG_DIR.exists():
        self.CONFIG_DIR.mkdir(parents=True, exist_ok=True)

    if not self.LOG_DIR.exists():
        self.LOG_DIR.mkdir(parents=True, exist_ok=True)

if __name__ == "__main__":
    print_logo()
    logger.info(f"AutoFilm {settings.APP_VERSION} 启动中...")

    # 启动API服务器
    if settings.API_ENABLE:
        from threading import Thread
        Thread(target=start_api_server, daemon=True).start()
        logger.info(f"API服务已启动在 {settings.API_PORT} 端口")

    scheduler = AsyncIOScheduler()

    if settings.AlistServerList:
        logger.info("检测到 Alist2Strm 模块配置，正在添加至后台任务")
        for server in settings.AlistServerList:
            cron = server.get("cron")
            if cron:
                scheduler.add_job(
                    Alist2Strm(**server).run, trigger=CronTrigger.from_crontab(cron)
                )
                logger.info(f"{server['id']} 已被添加至后台任务")
            else:
                logger.warning(f"{server['id']} 未设置 cron")
    else:
        logger.warning("未检测到 Alist2Strm 模块配置")

    if settings.Ani2AlistList:
        logger.info("检测到 Ani2Alist 模块配置，正在添加至后台任务")
        for server in settings.Ani2AlistList:
            cron = server.get("cron")
            if cron:
                scheduler.add_job(
                    Ani2Alist(**server).run, trigger=CronTrigger.from_crontab(cron)
                )
                logger.info(f"{server['id']} 已被添加至后台任务")
            else:
                logger.warning(f"{server['id']} 未设置 cron")
    else:
        logger.warning("未检测到 Ani2Alist 模块配置")

    if settings.LibraryPosterList:
        logger.info("检测到 LibraryPoster 模块配置，正在添加至后台任务")
        for poster in settings.LibraryPosterList:
            cron = poster.get("cron")
            if cron:
                scheduler.add_job(
                    LibraryPoster(**poster).run, trigger=CronTrigger.from_crontab(cron)
                )
                logger.info(f"{poster['id']} 已被添加至后台任务")
            else:
                logger.warning(f"{poster['id']} 未设置 cron")
    else:
        logger.warning("未检测到 LibraryPoster 模块配置")

    scheduler.start()
    logger.info("AutoFilm 启动完成")

    try:
        get_event_loop().run_forever()
    except (KeyboardInterrupt, SystemExit):
        logger.info("AutoFilm 程序退出！")
