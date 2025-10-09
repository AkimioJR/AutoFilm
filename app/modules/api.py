from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import APIKeyHeader

from app.core import settings, logger
from app.modules import Alist2Strm, Ani2Alist

router = APIRouter()
security = APIKeyHeader(name="X-API-Key", auto_error=False)

# 修改依赖声明方式（移除类型标注中的str）
async def get_api_key(api_key = Depends(security)):
    if not api_key:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Missing API Key"
        )
    if api_key != settings.API_KEY:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid API Key"
        )
    return api_key

# 修改路由参数声明（移除类型标注中的str）
@router.post("/trigger/alist2strm")
async def trigger_alist2strm(
    server_id: str,
    _ = Depends(get_api_key)
):
    logger.info(f"API触发Alist2Strm任务：{server_id}")
    # 查找对应配置
    server_config = next((s for s in settings.AlistServerList if s["id"] == server_id), None)
    if not server_config:
        raise HTTPException(status_code=404, detail="Server not found")

    await Alist2Strm(**server_config).run()
    return {"status": "success"}

@router.post("/trigger/ani2alist")
async def trigger_ani2alist(
    server_id: str,
    _ = Depends(get_api_key)
):
    logger.info(f"API触发Ani2Alist任务：{server_id}")
    server_config = next((s for s in settings.Ani2AlistList if s["id"] == server_id), None)
    if not server_config:
        raise HTTPException(status_code=404, detail="Server not found")

    await Ani2Alist(**server_config).run()
    return {"status": "success"}