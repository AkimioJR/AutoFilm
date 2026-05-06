from asyncio import sleep
from typing import Callable, AsyncGenerator
from time import time

from httpx import Response

from app.core import logger
from app.utils import RequestUtils, Multiton
from app.modules.alist.v3.path import AlistPath
from app.modules.alist.v3.storage import AlistStorage


class AlistClient(metaclass=Multiton):
    """
    Alist 客户端 API
    """

    def __init__(
        self,
        url: str,
        username: str = "",
        password: str = "",
        token: str = "",
    ) -> None:
        """
        AlistClient 类初始化

        :param url: Alist 服务器地址
        :param username: Alist 用户名
        :param password: Alist 密码
        :param token: Alist 永久令牌
        """

        if (username == "" or password == "") and token == "":
            raise ValueError("用户名及密码为空或令牌 Token 为空")

        self.__client = RequestUtils.get_client()
        self.__token = {
            "token": "",  # 令牌 token str
            "expires": 0,  # 令牌过期时间（时间戳，-1为永不过期） int
        }
        self.base_path = ""
        self.id = 0

        if not url.startswith("http"):
            url = "https://" + url
        self.url = url.rstrip("/")

        if token != "":
            self.__token["token"] = token
            self.__token["expires"] = -1
        elif username != "" and password != "":
            self.__username = str(username)
            self.___password = str(password)
        else:
            raise ValueError("用户名及密码为空或令牌 Token 为空")

        self.sync_api_me()

    async def __request(
        self,
        method: str,
        url: str,
        auth: bool = True,
        **kwargs,
    ) -> Response:
        """
        发送 HTTP 请求

        :param method 请求方法
        :param url 请求 url
        :param auth header 中是否带有 alist 认证令牌
        """

        if auth:
            headers = kwargs.get("headers", {})
            headers["Authorization"] = self.__get_token
            kwargs["headers"] = headers
        return await self.__client.request(method, url, **kwargs, sync=False)

    async def __get(self, url: str, auth: bool = True, **kwargs) -> Response:
        """
        发送 GET 请求

        :param url 请求 url
        :param auth header 中是否带有 alist 认证令牌
        """
        return await self.__request("get", url, auth, **kwargs)

    async def __post(self, url: str, auth: bool = True, **kwargs) -> Response:
        """
        发送 POST 请求

        :param url 请求 url
        :param auth header 中是否带有 alist 认证令牌
        """
        return await self.__request("post", url, auth, **kwargs)

    @property
    def username(self) -> str:
        """
        获取用户名
        """

        return self.__username

    @property
    def __password(self) -> str:
        """
        获取密码
        """

        return self.___password

    @property
    def __get_token(self) -> str:
        """
        返回可用登录令牌

        :return: 登录令牌 token
        """

        if self.__token["expires"] == -1:
            logger.debug("使用永久令牌")
            return self.__token["token"]
        else:
            logger.debug("使用临时令牌")
            now_stamp = int(time())

            if self.__token["expires"] < now_stamp:  # 令牌过期需要重新更新
                self.__token["token"] = self.api_auth_login()
                self.__token["expires"] = (
                    now_stamp + 2 * 24 * 60 * 60 - 5 * 60
                )  # 2天 - 5分钟（alist 令牌有效期为 2 天，提前 5 分钟刷新）

            return self.__token["token"]

    def api_auth_login(self) -> str:
        """
        登录 Alist 服务器认证账户信息

        :return: 重新申请的登录令牌 token
        """

        json = {"username": self.username, "password": self.__password}
        resp = self.__client.post(self.url + "/api/auth/login", json=json, sync=True)
        if resp.status_code != 200:
            raise RuntimeError(f"更新令牌请求发送失败，状态码：{resp.status_code}")

        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(f"更新令牌，错误信息：{result['message']}")

        logger.debug(f"{self.username} 更新令牌成功")
        return result["data"]["token"]

    def sync_api_me(self) -> None:
        """
        获取用户信息
        获取当前用户 base_path 和 id 并分别保存在 self.base_path 和 self.id 中
        """

        headers = {"Authorization": self.__get_token}
        resp = self.__client.get(self.url + "/api/me", headers=headers, sync=True)

        if resp.status_code != 200:
            raise RuntimeError(f"获取用户信息请求发送失败，状态码：{resp.status_code}")

        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(f"获取用户信息失败，错误信息：{result['message']}")

        try:
            self.base_path: str = result["data"]["base_path"]
            self.id: int = result["data"]["id"]
        except Exception:
            raise RuntimeError("获取用户信息失败")

    async def async_api_fs_list(self, dir_path: str) -> list[AlistPath]:
        """
        获取文件列表

        :param dir_path: 目录路径
        :return: AlistPath 对象列表
        """

        logger.debug(f"获取目录 {dir_path} 下的文件列表")

        json = {
            "path": dir_path,
            "password": "",
            "page": 1,
            "per_page": 0,
            "refresh": False,
        }

        resp = await self.__post(self.url + "/api/fs/list", json=json)
        if resp.status_code != 200:
            raise RuntimeError(
                f"获取目录 {dir_path} 的文件列表请求发送失败，状态码：{resp.status_code}"
            )

        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(
                f"获取目录 {dir_path} 的文件列表失败，错误信息：{result['message']}"
            )

        logger.debug(f"获取目录 {dir_path} 的文件列表成功")

        if result["data"]["total"] == 0:
            return []

        return [
            AlistPath(
                server_url=self.url,
                base_path=self.base_path,
                full_path=dir_path + "/" + alist_path["name"],
                **alist_path,
            )
            for alist_path in result["data"]["content"]
        ]

    async def async_api_fs_get(self, path: str) -> AlistPath:
        """
        获取文件/目录详细信息

        :param path: 文件/目录路径
        :return: AlistPath 对象
        """

        json = {
            "path": path,
            "password": "",
            "page": 1,
            "per_page": 0,
            "refresh": False,
        }

        resp = await self.__post(self.url + "/api/fs/get", json=json)
        if resp.status_code != 200:
            raise RuntimeError(
                f"获取路径 {path} 详细信息请求发送失败，状态码：{resp.status_code}"
            )
        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(
                f"获取路径 {path} 详细信息失败，详细信息：{result['message']}"
            )

        logger.debug(f"获取路径 {path} 详细信息成功")
        return AlistPath(
            server_url=self.url,
            base_path=self.base_path,
            full_path=path,
            **result["data"],
        )

    async def async_api_admin_storage_list(self) -> list[AlistStorage]:
        """
        列出存储列表 需要管理员用户权限

        :return: AlistStorage 对象列表
        """

        resp = await self.__get(self.url + "/api/admin/storage/list")
        if resp.status_code != 200:
            raise RuntimeError(
                f"获取存储器列表请求发送失败，状态码：{resp.status_code}"
            )

        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(f"获取存储器列表失败，详细信息：{result['message']}")

        logger.debug("获取存储器列表成功")
        return [AlistStorage(**storage) for storage in result["data"]["content"]]

    async def async_api_admin_storage_create(self, storage: AlistStorage) -> None:
        """
        创建存储 需要管理员用户权限

        :param storage: AlistStorage 对象
        """

        json = {
            "mount_path": storage.mount_path,
            "order": storage.order,
            "remark": storage.remark,
            "cache_expiration": storage.cache_expiration,
            "web_proxy": storage.web_proxy,
            "webdav_policy": storage.webdav_policy,
            "down_proxy_url": storage.down_proxy_url,
            "enable_sign": storage.enable_sign,
            "driver": storage.driver,
            "order_by": storage.order_by,
            "order_direction": storage.order_direction,
            "addition": storage.addition,
        }

        resp = await self.__post(self.url + "/api/admin/storage/create", json=json)
        if resp.status_code != 200:
            raise RuntimeError(f"创建存储请求发送失败，状态码：{resp.status_code}")
        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(f"创建存储失败，详细信息：{result['message']}")

        logger.debug("创建存储成功")
        return

    async def async_api_admin_storage_update(self, storage: AlistStorage) -> None:
        """
        更新存储，需要管理员用户权限

        :param storage: AlistStorage 对象
        """

        json = {
            "id": storage.id,
            "mount_path": storage.mount_path,
            "order": storage.order,
            "driver": storage.driver,
            "cache_expiration": storage.cache_expiration,
            "status": storage.status,
            "addition": storage.addition,
            "remark": storage.remark,
            "modified": storage.modified,
            "disabled": storage.disabled,
            "enable_sign": storage.enable_sign,
            "order_by": storage.order_by,
            "order_direction": storage.order_direction,
            "extract_folder": storage.extract_folder,
            "web_proxy": storage.web_proxy,
            "webdav_policy": storage.webdav_policy,
            "down_proxy_url": storage.down_proxy_url,
        }

        resp = await self.__post(self.url + "/api/admin/storage/update", json=json)
        if resp.status_code != 200:
            raise RuntimeError(f"更新存储请求发送失败，状态码：{resp.status_code}")

        result = resp.json()

        if result["code"] != 200:
            raise RuntimeError(f"更新存储器失败，详细信息：{result['message']}")

        logger.debug(
            f"更新存储器成功，存储器ID：{storage.id}，挂载路径：{storage.mount_path}"
        )
        return

    async def iter_path(
        self,
        dir_path: str,
        wait_time: float | int,
        is_detail: bool = True,
        filter: Callable[[AlistPath], bool] = lambda x: True,
    ) -> AsyncGenerator[AlistPath, None]:
        """
        异步路径列表生成器
        返回目录及其子目录的所有文件和目录的 AlistPath 对象

        :param dir_path: 目录路径
        :param wait_time: 每轮遍历等待时间（单位秒）,
        :param is_detail：是否获取详细信息（raw_url）
        :param filter: 匿名函数过滤器（默认不启用）
        :return: AlistPath 对象生成器
        """

        for path in await self.async_api_fs_list(dir_path):
            await sleep(wait_time)
            if path.is_dir:
                async for child_path in self.iter_path(
                    dir_path=path.full_path,
                    wait_time=wait_time,
                    is_detail=is_detail,
                    filter=filter,
                ):
                    yield child_path

            if filter(path):
                if is_detail:
                    yield await self.async_api_fs_get(path.full_path)
                else:
                    yield path

    async def get_storage_by_mount_path(
        self, mount_path: str, create: bool = False, **kwargs
    ) -> AlistStorage | None:
        """
        通过挂载路径获取存储器信息

        :param mount_path: 挂载路径
        :param create: 未找到存储器时是否创建
        :param kwargs: 创建存储器 AlistStorge 时的参数
        :return: AlistStorage 对象
        """

        for storage in await self.async_api_admin_storage_list():
            if storage.mount_path == mount_path:
                return storage
        logger.debug(f"在 Alist 服务器上未找到存储器 {mount_path}")

        if create:
            kwargs["mount_path"] = mount_path
            storage = AlistStorage(**kwargs)
            await self.async_api_admin_storage_create(storage)
            return storage

        return None