import random
from pathlib import Path
from io import BytesIO
from typing import Any, AsyncGenerator

from PIL import Image, ImageDraw, ImageFont

from app.core import logger
from app.utils import RequestUtils, PhotoUtils


class LibraryPoster:
    def __init__(
        self,
        url: str,
        api_key: str,
        title_font_path: str,
        subtitle_font_path: str,
        configs: list[dict[str, str]],
        **_,
    ) -> None:
        """
        初始化库海报更新客户端
        :param url: 服务器地址
        :param api_key: API 密钥
        :param title_font_path: 主标题字体路径
        :param subtitle_font_path: 副标题字体路径
        """
        self.__server_url = url
        self.__api_key = api_key
        self.__title_font_path = Path(title_font_path)
        self.__subtitle_font_path = Path(subtitle_font_path)
        self.__configs = configs

    async def get_users(self) -> list[dict[str, Any]]:
        """
        获取用户列表
        :return: 用户列表
        """
        resp = await RequestUtils.get(
            f"{self.__server_url}/Users?api_key={self.__api_key}"
        )
        if resp is None:
            logger.warning(f"获取 {self.__server_url} 用户列表失败")
            return []

        if resp.status_code != 200:
            logger.warning(
                f"获取 {self.__server_url} 用户列表失败, 状态码: {resp.status_code}"
            )
            return []
        return resp.json()

    async def get_libraries(self) -> list[dict[str, Any]]:
        """
        返回媒体库列表
        """
        resp = await RequestUtils.get(
            f"{self.__server_url}/Library/MediaFolders?api_key={self.__api_key}",
        )
        if resp is None:
            logger.warning(f"获取 {self.__server_url} 媒体库列表失败")
            return []

        if resp.status_code != 200:
            logger.warning(
                f"获取 {self.__server_url} 媒体库列表失败, 状态码: {resp.status_code}"
            )
            return []

        return resp.json()["Items"]

    async def get_library_items(
        self,
        library_id: str,
        user_id: str = "",
    ) -> AsyncGenerator[dict[str, Any], None]:
        """
        获取指定媒体库的详细信息
        :param library_id: 媒体库 ID
        :param user_id: 用户 ID（可选）
        :return: 媒体库项目列表
        """

        async def fetch_items(
            parent_id: str, user_id: str, max_depth: int = 1, current_depth: int = 0
        ) -> AsyncGenerator[dict[str, Any], None]:
            """
            递归获取媒体库项目
            """
            url = f"{self.__server_url}/Users/{user_id}/Items?ParentId={parent_id}&api_key={self.__api_key}"
            resp = await RequestUtils.get(url)

            if not resp or resp.status_code != 200:
                return
            for item in resp.json().get("Items", []):
                if item.get("IsFolder", False) and current_depth < max_depth:
                    # 递归获取子项
                    async for sub_item in fetch_items(
                        parent_id=item["Id"],
                        user_id=user_id,
                        max_depth=max_depth,
                        current_depth=current_depth + 1,
                    ):
                        yield sub_item
                else:
                    yield item

        if not user_id:
            users = await self.get_users()
            if not users:
                raise Exception("未找到任何用户，无法获取媒体库项目")
            user_id = users[0]["Id"]  # 默认使用第一个用户

        url = f"{self.__server_url}/Users/{user_id}/Items?ParentId={library_id}&api_key={self.__api_key}"
        resp = await RequestUtils.get(url)

        if resp is None or resp.status_code != 200:
            raise Exception(
                f"获取 {library_id} 媒体库信息失败, 状态码: {resp.status_code if resp else '无响应'}"
            )

        return fetch_items(library_id, user_id, 5)

    async def download_item_image(
        self,
        item: dict[str, Any],
        image_type: str = "Primary",
    ) -> Image.Image | None:
        """
        下载指定项目海报图片
        :param item: 项目字典
        :return: 图片字节内容
        """
        url = f"{self.__server_url}/Items/{item['Id']}/Images/{image_type}?api_key={self.__api_key}"
        resp = await RequestUtils.get(url)

        if resp is None or resp.status_code != 200:
            logger.warning(
                f"下载项目 {item['Name']} {image_type} 类型图片失败, 状态码: {resp.status_code if resp else '无响应'}"
            )
            return None

        return Image.open(BytesIO(resp.content))

    async def download_library_poster(
        self,
        library: dict[str, Any],
    ) -> AsyncGenerator[Image.Image, None]:
        """
        下载媒体库海报
        :param library: 媒体库字典
        :return:
        """
        logger.info(f"开始下载 {library['Name']} 媒体库的海报图片")
        async for item in self.get_library_items(library["Id"]):
            image = await self.download_item_image(item)
            if image is not None:
                yield image

    def process_poster(
        self,
        images: list[Image.Image],
        title: str = "",
        subtitle: str = "",
        width: int = 1920,
        height: int = 1080,
    ) -> Image.Image:
        """
        处理海报图片，将图片布局在右半边
        :param images: 图片列表
        :param title: 海报标题
        :param subtitle: 海报副标题
        :param width: 背景宽度
        :param height: 背景高度
        :return: 处理后的海报图片
        """

        logger.info(f"开始处理海报图片，标题: {title}, 副标题: {subtitle}")
        # 随机打乱图片顺序
        random.shuffle(images)

        # 布局参数
        COLS = 3
        ROWS = 3

        # 动态计算图片尺寸（相对于背景尺寸）
        CELL_WIDTH = int(width * 0.20)  # 约占背景宽度的20%
        CELL_HEIGHT = int(CELL_WIDTH * 1.5)  # 保持海报比例 2:3

        # 动态计算间距
        COLUMN_SPACING = int(width * 0.025)  # 列间距约占2.5%
        ROW_SPACING = int(height * 0.05)  # 行间距约占5%

        CORNER_RADIUS = max(15, int(CELL_WIDTH * 0.08))  # 圆角半径
        ROTATION_ANGLE = -18  # 旋转角度
        RIGHT_MARGIN = int(width * 0.05)  # 右边距占5%

        # 根据图片大小自适应计算阴影参数
        # 文字阴影偏移：基于字体大小和背景尺寸
        text_shadow_offset_x = max(2, int(width * 0.002))  # 最小2px，约占宽度的0.2%
        text_shadow_offset_y = max(2, int(height * 0.003))  # 最小2px，约占高度的0.3%
        text_shadow_offset = (text_shadow_offset_x, text_shadow_offset_y)

        # 图片阴影参数：基于图片尺寸
        img_shadow_offset_x = max(3, int(CELL_WIDTH * 0.015))
        img_shadow_offset_y = max(3, int(CELL_HEIGHT * 0.012))
        img_shadow_offset = (img_shadow_offset_x, img_shadow_offset_y)
        img_shadow_blur = max(
            2, int(min(CELL_WIDTH, CELL_HEIGHT) * 0.012)
        )  # 模糊半径基于图片较小边

        # 获取主题色并创建背景
        theme_color, text_color = PhotoUtils.get_primary_color(random.choice(images))
        background = PhotoUtils.create_gradient_background(width, height, theme_color)

        draw = ImageDraw.Draw(background)

        title_font_size = int(height * 0.15)
        subtitle_font_size = int(height * 0.06)

        try:
            title_font = ImageFont.truetype(
                self.__title_font_path.as_posix(), size=title_font_size
            )
            subtitle_font = ImageFont.truetype(
                self.__subtitle_font_path.as_posix(), size=subtitle_font_size
            )
        except Exception as e:
            logger.warning(f"加载自定义字体失败: {e}")
            logger.warning(
                f"主标题字体路径: {self.__title_font_path} (存在: {self.__title_font_path.exists()})"
            )
            logger.warning(
                f"副标题字体路径: {self.__subtitle_font_path} (存在: {self.__subtitle_font_path.exists()})"
            )
            title_font = ImageFont.load_default()
            subtitle_font = ImageFont.load_default()

        # 主标题在左侧中间偏上
        title_bbox = draw.textbbox((0, 0), title, font=title_font)
        title_text_w = title_bbox[2] - title_bbox[0]
        left_half_center = width // 4  # 左半边的中心位置
        title_x = left_half_center - title_text_w // 2
        title_y = int(height * 0.35)  # 中间偏上

        PhotoUtils.draw_text_on_image(
            background,
            title,
            (title_x, title_y),
            title_font,
            text_color,
            shadow_enabled=False,
            shadow_offset=text_shadow_offset,
        )

        # 副标题在左侧中间偏下
        subtitle_bbox = draw.textbbox((0, 0), subtitle, font=subtitle_font)
        subtitle_text_w = subtitle_bbox[2] - subtitle_bbox[0]
        subtitle_x = left_half_center - subtitle_text_w // 2
        subtitle_y = int(height * 0.60)  # 中间偏下

        PhotoUtils.draw_text_on_image(
            background,
            subtitle,
            (subtitle_x, subtitle_y),
            subtitle_font,
            text_color,
            shadow_enabled=True,
            shadow_offset=text_shadow_offset,
        )

        # 计算右半边区域
        right_half_start = width // 2
        available_width = width // 2 - RIGHT_MARGIN

        # 计算网格尺寸（允许超出右半边）
        grid_width = COLS * CELL_WIDTH + (COLS - 1) * COLUMN_SPACING
        grid_height = ROWS * CELL_HEIGHT + (ROWS - 1) * ROW_SPACING

        # 网格起始位置 - 让中心图片在右半边居中，周围可以超出
        grid_center_x = right_half_start + int(available_width * 0.7)
        grid_center_y = height // 2

        grid_start_x = grid_center_x - grid_width // 2
        grid_start_y = grid_center_y - grid_height // 2

        # 处理每个海报
        processed_count = 0
        for col in range(COLS):
            for row in range(ROWS):
                if processed_count >= len(images):
                    break

                img = images[processed_count]
                processed_count += 1

                # 1. 调整图片大小
                resized = img.resize((CELL_WIDTH, CELL_HEIGHT), Image.LANCZOS)

                # 2. 应用圆角
                rounded = PhotoUtils.apply_rounded_corners(resized, CORNER_RADIUS)

                # 3. 添加阴影
                shadowed = PhotoUtils.add_shadow(
                    rounded, offset=img_shadow_offset, blur_radius=img_shadow_blur
                )

                # 4. 计算基础位置 - 让每列的中心点在一条斜线上
                # 原始网格中心位置
                original_center_x = (
                    grid_start_x + col * (CELL_WIDTH + COLUMN_SPACING) + CELL_WIDTH // 2
                )
                original_center_y = (
                    grid_start_y + row * (CELL_HEIGHT + ROW_SPACING) + CELL_HEIGHT // 2
                )

                # 根据旋转角度计算水平偏移，使斜线效果在旋转后仍然保持
                vertical_offset = height * col * 0.03
                target_center_y = original_center_y + vertical_offset

                # 5. 应用旋转
                rotated = shadowed.rotate(
                    ROTATION_ANGLE,
                    expand=True,
                    fillcolor=(0, 0, 0, 0),
                    resample=Image.BICUBIC,
                )

                # 6. 计算旋转后的最终位置，确保旋转中心对齐
                pos_x = original_center_x - rotated.width / 2
                pos_y = target_center_y - rotated.height / 2

                # 7. 粘贴到背景
                background.paste(rotated, (int(pos_x), int(pos_y)), rotated)

        logger.info(f"成功处理 {processed_count} 张海报图片")
        return background

    async def update_library_image(
        self, library: dict[str, Any], image: Image.Image, image_type: str = "Primary"
    ) -> None:
        """
        更新媒体库的海报图片
        :param library: 媒体库字典
        :param image: 要更新的图片
        :param image_type: 图片类型，默认为 Primary
        """
        url = f"{self.__server_url}/Items/{library['Id']}/Images/{image_type}?api_key={self.__api_key}"
        headers = {
            "Content-Type": "image/png",
        }

        image_data_base64 = PhotoUtils.encode_image(image=image, format="PNG")
        resp = await RequestUtils.post(url, data=image_data_base64, headers=headers)
        if resp is None or resp.status_code != 204:
            logger.warning(
                f"更新 {library['Name']} 媒体库图片失败, 状态码: {resp.status_code if resp else '无响应'}"
            )
        else:
            logger.info(f"成功更新 {library['Name']} 媒体库 {image_type} 类型的图片")

    async def process_library(
        self,
        library: dict[str, Any],
        title: str = "",
        subtitle: str = "",
        limit: int = 15,
    ) -> None:
        """
        处理单个媒体库
        :param library: 媒体库项目字典
        :param title: 海报标题
        :param subtitle: 海报副标题
        :param limit: 限制下载的图片数量
        """

        images: list[Image.Image] = []
        async for image in self.download_library_poster(library):
            images.append(image)
            if len(images) >= limit:
                break

        logger.info(f"获取到 {library['Name']} 媒体库的 {len(images)} 张海报图片")
        result = self.process_poster(images, title, subtitle)
        await self.update_library_image(library, result)
        logger.info(f"媒体库 {library['Name']} 的海报图片处理成功")

    async def run(self) -> None:
        """
        执行库海报更新
        """
        libraries = await self.get_libraries()
        library_kv: dict[str, str] = {item["Name"]: item for item in libraries}
        for config in self.__configs:
            if config["library_name"] in library_kv:
                await self.process_library(
                    library_kv[config["library_name"]],
                    config.get("title", ""),
                    config.get("subtitle", ""),
                )
