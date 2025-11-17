from asyncio import to_thread, Semaphore, TaskGroup
from os import PathLike
from pathlib import Path
from re import compile as re_compile
import traceback

from aiofile import async_open

from app.core import logger
from app.utils import RequestUtils
from app.extensions import VIDEO_EXTS, SUBTITLE_EXTS, IMAGE_EXTS, NFO_EXTS
from app.modules.alist import AlistClient, AlistPath
from app.modules.alist2strm.mode import Alist2StrmMode

class Alist2Strm:
    def __init__(
        self,
        id: str = "",
        url: str = "http://localhost:5244",
        username: str = "",
        password: str = "",
        token: str = "",
        public_url: str = "",
        source_dir: str = "/",
        target_dir: str | PathLike = "",
        flatten_mode: bool = False,
        subtitle: bool = False,
        image: bool = False,
        nfo: bool = False,
        mode: str = "AlistURL",
        overwrite: bool = False,
        other_ext: str = "",
        max_workers: int = 50,
        max_downloaders: int = 5,
        wait_time: float | int = 0,
        sync_server: bool = False,
        sync_ignore: str | None = None,
        smart_protection: dict | None = None,
        **_,
    ) -> None:
        """
        实例化 Alist2Strm 对象

        :param id: 任务标识 ID，默认为空
        :param url: Alist 服务器地址，默认为 "http://localhost:5244"
        :param username: Alist 用户名，默认为空
        :param password: Alist 密码，默认为空
        :param public_url: 公共访问地址，用于生成 .strm 文件（可选）
        :param source_dir: 需要同步的 Alist 的目录，默认为 "/"
        :param target_dir: strm 文件输出目录，默认为当前工作目录
        :param flatten_mode: 平铺模式，将所有 Strm 文件保存至同一级目录，默认为 False
        :param subtitle: 是否下载字幕文件，默认为 False
        :param image: 是否下载图片文件，默认为 False
        :param nfo: 是否下载 .nfo 文件，默认为 False
        :param mode: Strm模式(AlistURL/RawURL/AlistPath)
        :param overwrite: 本地路径存在同名文件时是否重新生成/下载该文件，默认为 False
        :param sync_server: 是否同步服务器，启用后若服务器中删除了文件，也会将本地文件删除，默认为 True
        :param other_ext: 自定义下载后缀，使用西文半角逗号进行分割，默认为空
        :param max_workers: 最大并发数
        :param max_downloaders: 最大同时下载
        :param wait_time: 遍历请求间隔时间，单位为秒，默认为 0
        :param sync_ignore: 同步时忽略的文件正则表达式
        :param smart_protection: 智能保护配置 {enabled: bool, threshold: int, grace_scans: int}
        """

        self.client = AlistClient(url, username, password, token, public_url)
        self.mode = Alist2StrmMode.from_str(mode)

        self.source_dir = source_dir
        self.target_dir = Path(target_dir)

        self.flatten_mode = flatten_mode
        if flatten_mode:
            subtitle = image = nfo = False

        download_exts: set[str] = set()
        if subtitle:
            download_exts |= SUBTITLE_EXTS
        if image:
            download_exts |= IMAGE_EXTS
        if nfo:
            download_exts |= NFO_EXTS
        if other_ext:
            download_exts |= frozenset(other_ext.lower().split(","))

        self.download_exts = download_exts
        self.process_file_exts = VIDEO_EXTS | download_exts

        self.overwrite = overwrite
        self.__max_workers = Semaphore(max_workers)
        self.__max_downloaders = Semaphore(max_downloaders)
        self.wait_time = wait_time
        self.sync_server = sync_server

        if sync_ignore:
            self.sync_ignore_pattern = re_compile(sync_ignore)
        else:
            self.sync_ignore_pattern = None
        
        if smart_protection and smart_protection.get('enabled', False):
            from app.modules.alist2strm.strm_protection import StrmProtectionManager
            threshold = smart_protection.get('threshold', 100)
            grace_scans = smart_protection.get('grace_scans', 3)
            self.strm_protection = StrmProtectionManager(
                self.target_dir, id, threshold, grace_scans
            )
            logger.info(f".strm 保护已启用：阈值={threshold}，宽限期={grace_scans}")
        else:
            self.strm_protection = None

    async def run(self) -> None:
        """
        处理主体
        """
        
        # BDMV 处理相关变量初始化
        self.bdmv_collections: dict[str, list[tuple[AlistPath, int]]] = {}  # BDMV目录 -> [(文件路径, 文件大小)]
        self.bdmv_largest_files: dict[str, AlistPath] = {}  # BDMV目录 -> 最大文件路径

        def filter(path: AlistPath) -> bool:
            """
            过滤器
            根据 Alist2Strm 配置判断是否需要处理该文件
            将云盘上上的文件对应的本地文件路径保存至 self.processed_local_paths

            :param path: AlistPath 对象
            """

            if path.is_dir:
                return False

            # 跳过系统文件夹和不需要的文件
            if any(folder in path.full_path for folder in ["@eaDir", "Thumbs.db", ".DS_Store"]):
                return False

            # 完全跳过 BDMV 文件夹内的所有文件（除了我们特殊处理的 .m2ts 文件）
            if "/BDMV/" in path.full_path and not self._is_bdmv_file(path):
                logger.debug(f"跳过 BDMV 文件夹内的文件: {path.name}")
                return False

            if path.suffix.lower() not in self.process_file_exts:
                logger.debug(f"文件 {path.name} 不在处理列表中")
                return False

            # 检查是否为 BDMV 文件
            if self._is_bdmv_file(path):
                self._collect_bdmv_file(path)
                # 暂时不处理，等收集完所有文件后再决定
                return False

            try:
                local_path = self.__get_local_path(path)
            except OSError as e:  # 可能是文件名过长
                logger.warning(f"获取 {path.full_path} 本地路径失败：{e}")
                return False

            self.processed_local_paths.add(local_path)

            if not self.overwrite and local_path.exists():
                if path.suffix in self.download_exts:
                    local_path_stat = local_path.stat()
                    if local_path_stat.st_mtime < path.modified_timestamp:
                        logger.debug(
                            f"文件 {local_path.name} 已过期，需要重新处理 {path.full_path}"
                        )
                        return True
                    if local_path_stat.st_size < path.size:
                        logger.debug(
                            f"文件 {local_path.name} 大小不一致，可能是本地文件损坏，需要重新处理 {path.full_path}"
                        )
                        return True
                logger.debug(
                    f"文件 {local_path.name} 已存在，跳过处理 {path.full_path}"
                )
                return False

            return True


        if self.mode == Alist2StrmMode.RawURL:
            is_detail = True
        else:
            is_detail = False

        self.processed_local_paths = set()  # 云盘文件对应的本地文件路径

        # 第一阶段：收集所有文件信息并直接处理普通文件
        async with self.__max_workers, TaskGroup() as tg:
            async for path in self.client.iter_path(
                dir_path=self.source_dir,
                wait_time=self.wait_time,
                is_detail=is_detail,
                filter=filter,
            ):
                # 直接处理普通文件，不需要额外的 list
                tg.create_task(self.__file_processer(path))

        # 完成 BDMV 文件收集，确定最大文件
        self._finalize_bdmv_collections()
        
        # 第二阶段：处理 BDMV 最大文件
        logger.info(f"开始处理 {len(self.bdmv_largest_files)} 个 BDMV 目录")
        for bdmv_root, largest_file in self.bdmv_largest_files.items():
            try:
                logger.info(f"处理 BDMV 目录: {bdmv_root}")
                logger.info(f"最大文件: {largest_file.full_path}")
                
                # 重新获取详细信息以确保有 raw_url
                if self.mode == Alist2StrmMode.RawURL and not largest_file.raw_url:
                    logger.debug(f"重新获取 BDMV 文件详细信息: {largest_file.full_path}")
                    try:
                        updated_path = await self.client.async_api_fs_get(largest_file.full_path)
                        # 保持原有的 full_path，只更新其他属性
                        original_full_path = largest_file.full_path
                        largest_file = updated_path
                        largest_file.full_path = original_full_path
                    except Exception as e:
                        logger.warning(f"重新获取 BDMV 文件详细信息失败: {e}")
                
                # 处理文件
                await self.__file_processer(largest_file)
                
                # 添加到已处理路径列表
                local_path = self.__get_local_path(largest_file)
                self.processed_local_paths.add(local_path)
                
                logger.info(f"BDMV 文件处理完成: {largest_file.name}")
            except Exception as e:
                logger.error(f"处理 BDMV 文件 {largest_file.full_path} 时出错：{e}")
                logger.error(f"详细错误信息: {traceback.format_exc()}")
                continue

        if self.sync_server:
            await self.__cleanup_local_files()
            logger.info("清理过期的 .strm 文件完成")
        logger.info("Alist2Strm 处理完成")

    async def __file_processer(self, path: AlistPath) -> None:
        """
        异步保存文件至本地

        :param path: AlistPath 对象
        """
        local_path = self.__get_local_path(path)
        logger.debug(f"__file_processer: 处理文件 {path.full_path} -> 本地路径 {local_path} | 模式 {self.mode}")

        # 统一的 URL 生成逻辑，BDMV 文件与普通文件使用相同的逻辑
        if self.mode == Alist2StrmMode.AlistURL:
            content = path.download_url
        elif self.mode == Alist2StrmMode.RawURL:
            content = path.raw_url
        elif self.mode == Alist2StrmMode.AlistPath:
            content = path.full_path

        logger.debug(f"__file_processer: 初始 content = {content}")

        if not content:
            logger.warning(f"文件 {path.full_path} 的内容为空，跳过处理")
            return

        await to_thread(local_path.parent.mkdir, parents=True, exist_ok=True)

        logger.debug(f"开始处理 {local_path} | 内容: {content}")
        if local_path.suffix == ".strm":
            async with async_open(local_path, mode="w", encoding="utf-8") as file:
                await file.write(content)
            logger.info(f"{local_path.name} 创建成功")
        else:
            async with self.__max_downloaders:
                await RequestUtils.download(path.download_url, local_path)
                logger.info(f"{local_path.name} 下载成功")

    def __get_local_path(self, path: AlistPath) -> Path:
        """
        根据给定的 AlistPath 对象和当前的配置，计算出本地文件路径。

        :param path: AlistPath 对象
        :return: 本地文件路径
        """
        # 检查是否为 BDMV 文件
        if self._is_bdmv_file(path):
            bdmv_root = self._get_bdmv_root_dir(path)
            if bdmv_root and self._should_process_bdmv_file(path):
                # 为 BDMV 文件生成特殊路径
                movie_title = self._get_movie_title_from_bdmv_path(bdmv_root)
                
                if self.flatten_mode:
                    local_path = self.target_dir / f"{movie_title}.strm"
                else:
                    # 计算相对于 source_dir 的路径
                    relative_path = bdmv_root.replace(self.source_dir, "", 1)
                    if relative_path.startswith("/"):
                        relative_path = relative_path[1:]
                    
                    # 将 .strm 文件放在电影根目录下，使用电影标题命名
                    local_path = self.target_dir / relative_path / f"{movie_title}.strm"
                
                return local_path

        # 原有逻辑保持不变
        if self.flatten_mode:
            local_path = self.target_dir / path.name
        else:
            relative_path = path.full_path.replace(self.source_dir, "", 1)
            if relative_path.startswith("/"):
                relative_path = relative_path[1:]
            local_path = self.target_dir / relative_path

        if path.suffix.lower() in VIDEO_EXTS:
            local_path = local_path.with_suffix(".strm")

        return local_path

    async def __cleanup_local_files(self) -> None:
        """
        删除服务器中已删除的本地的 .strm 文件及其关联文件
        如果文件后缀在 sync_ignore 中，则不会被删除
        """
        logger.info("开始清理本地文件")

        if self.flatten_mode:
            all_local_files = [f for f in self.target_dir.iterdir() if f.is_file()]
        else:
            all_local_files = [f for f in self.target_dir.rglob("*") if f.is_file()]

        files_to_delete = set(all_local_files) - self.processed_local_paths
        strm_present = None
        if self.strm_protection:
            strm_present = {f for f in self.processed_local_paths if f.suffix == '.strm'}
        
        if not files_to_delete:
            if self.strm_protection:
                self.strm_protection.process(set(), strm_present)
                self.strm_protection.save()
            return
        
        strm_to_delete = {f for f in files_to_delete if f.suffix == '.strm'}
        other_files = files_to_delete - strm_to_delete
        
        if self.strm_protection:
            strm_to_delete = self.strm_protection.process(strm_to_delete, strm_present)
            self.strm_protection.save()
        
        files_to_delete = strm_to_delete | other_files
        
        for file_path in files_to_delete:
            # 检查文件是否匹配忽略正则表达式
            if self.sync_ignore_pattern and self.sync_ignore_pattern.search(
                file_path.name
            ):
                logger.debug(f"文件 {file_path.name} 在忽略列表中，跳过删除")
                continue

            try:
                if file_path.exists():
                    await to_thread(file_path.unlink)
                    logger.info(f"删除文件：{file_path}")

                    # 检查并删除空目录
                    parent_dir = file_path.parent
                    while parent_dir != self.target_dir:
                        if any(parent_dir.iterdir()):
                            break  # 目录不为空，跳出循环
                        else:
                            parent_dir.rmdir()
                            logger.info(f"删除空目录：{parent_dir}")
                        parent_dir = parent_dir.parent
            except Exception as e:
                logger.error(f"删除文件 {file_path} 失败：{e}")

    def _is_bdmv_file(self, path: AlistPath) -> bool:
        """
        检查文件是否为 BDMV 结构中的 .m2ts 文件
        
        :param path: AlistPath 对象
        :return: 是否为 BDMV 文件
        """
        return "/BDMV/STREAM/" in path.full_path and path.suffix.lower() == ".m2ts"

    def _get_bdmv_root_dir(self, path: AlistPath) -> str:
        """
        获取 BDMV 文件的根目录路径
        
        :param path: BDMV 中的文件路径
        :return: BDMV 根目录路径
        """
        full_path = path.full_path
        bdmv_index = full_path.find("/BDMV/")
        if bdmv_index != -1:
            return full_path[:bdmv_index]
        return ""

    def _get_movie_title_from_bdmv_path(self, bdmv_root: str) -> str:
        """
        从 BDMV 根目录路径提取电影标题
        
        :param bdmv_root: BDMV 根目录路径
        :return: 电影标题
        """
        # 获取最后一个目录名作为电影标题
        return Path(bdmv_root).name

    def _collect_bdmv_file(self, path: AlistPath) -> None:
        """
        收集 BDMV 文件信息
        
        :param path: BDMV 中的 .m2ts 文件路径
        """
        bdmv_root = self._get_bdmv_root_dir(path)
        if not bdmv_root:
            return

        if bdmv_root not in self.bdmv_collections:
            self.bdmv_collections[bdmv_root] = []

        # 添加文件信息到集合中
        self.bdmv_collections[bdmv_root].append((path, path.size))
        logger.debug(f"收集 BDMV 文件: {path.full_path}, 大小: {path.size}")

    def _finalize_bdmv_collections(self) -> None:
        """
        完成 BDMV 文件收集，确定每个 BDMV 目录中的最大文件
        """
        for bdmv_root, files in self.bdmv_collections.items():
            if not files:
                continue

            movie_title = self._get_movie_title_from_bdmv_path(bdmv_root)
            logger.info(f"BDMV 目录 '{movie_title}' 中发现 {len(files)} 个 .m2ts 文件:")
            
            # 按大小排序并显示所有文件
            sorted_files = sorted(files, key=lambda x: x[1], reverse=True)
            for i, (file_path, file_size) in enumerate(sorted_files):
                size_mb = file_size / (1024 * 1024)
                status = "✓ 选中" if i == 0 else "  跳过"
                logger.info(f"  {status} {file_path.name}: {size_mb:.1f} MB ({file_size} 字节)")

            # 找出最大的文件
            largest_file = max(files, key=lambda x: x[1])
            self.bdmv_largest_files[bdmv_root] = largest_file[0]
            
            largest_size_mb = largest_file[1] / (1024 * 1024)
            logger.info(f"BDMV 目录 '{movie_title}' 最终选择: {largest_file[0].name} ({largest_size_mb:.1f} MB)")

    def _should_process_bdmv_file(self, path: AlistPath) -> bool:
        """
        检查 BDMV 文件是否应该被处理（即是否为最大文件）
        
        :param path: BDMV 中的 .m2ts 文件路径
        :return: 是否应该处理
        """
        bdmv_root = self._get_bdmv_root_dir(path)
        if not bdmv_root:
            return False

        largest_file = self.bdmv_largest_files.get(bdmv_root)
        return largest_file is not None and largest_file.full_path == path.full_path

