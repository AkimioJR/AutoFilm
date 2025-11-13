from pathlib import Path
from datetime import datetime
import json

from app.core import logger


class StrmProtectionManager:
    """使用计数器系统的文件保护"""
    
    def __init__(self, target_dir: Path, task_id: str, threshold: int, grace_scans: int):
        self.target_dir = target_dir
        self.state_file = target_dir / f".autofilm_strm_{task_id}.json"
        self.threshold = threshold
        self.grace_scans = grace_scans
        self.protected = self._load()
    
    def _to_relative(self, file_path: Path) -> str:
        """将绝对路径转换为相对于 target_dir 的相对路径"""
        return file_path.relative_to(self.target_dir).as_posix()
    
    def _to_absolute(self, rel_path: str) -> Path:
        """将相对路径转换为绝对路径"""
        return self.target_dir / rel_path
    
    def _load(self) -> dict:
        """加载状态 {文件: 计数器}"""
        if self.state_file.exists():
            try:
                with open(self.state_file, 'r', encoding='utf-8') as f:
                    return json.load(f).get("protected", {})
            except (json.JSONDecodeError, IOError, KeyError) as e:
                logger.warning(f"加载保护状态失败：{e}，重新开始")
        return {}
    
    def save(self) -> None:
        """使用原子写入将状态保存到磁盘"""
        temp_file = self.state_file.with_suffix('.tmp')
        try:
            with open(temp_file, 'w', encoding='utf-8') as f:
                json.dump({
                    "updated": datetime.now().isoformat(),
                    "protected": self.protected
                }, f, indent=2, ensure_ascii=False)
            temp_file.replace(self.state_file)
        except Exception as e:
            logger.error(f"保护状态保存失败：{e}")
            if temp_file.exists():
                temp_file.unlink()
    
    def process(self, strm_to_delete: set[Path], strm_present: set[Path]) -> set[Path]:
        """
        处理 .strm 文件并返回现在要删除的文件
        
        :param strm_to_delete: Alist 中不存在的 .strm 文件
        :param strm_present: Alist 中存在的 .strm 文件
        :return: 现在要删除的文件
        """
        returned = 0
        for rel_path in list(self.protected.keys()):
            abs_path = self._to_absolute(rel_path)
            if abs_path in strm_present:
                del self.protected[rel_path]
                returned += 1
        
        if returned > 0:
            logger.info(f"{returned} 个 .strm 文件已恢复，取消保护")
        
        if len(strm_to_delete) < self.threshold:
            if len(strm_to_delete) > 0:
                logger.info(f"正常删除 {len(strm_to_delete)} 个 .strm（阈值：{self.threshold}）")
            return strm_to_delete
        
        logger.warning(f"保护激活：{len(strm_to_delete)} 个 .strm 待删除（阈值：{self.threshold}）")
        
        for file_path in strm_to_delete:
            rel_path = self._to_relative(file_path)
            self.protected[rel_path] = self.protected.get(rel_path, 0) + 1
        
        ready_rel = {
            rel_path for rel_path, count in self.protected.items() 
            if count >= self.grace_scans
        }
        
        pending = len(self.protected) - len(ready_rel)
        
        if ready_rel:
            logger.warning(f"删除 {len(ready_rel)} 个 .strm（经过 {self.grace_scans} 次扫描确认）")
            ready = {self._to_absolute(rel_path) for rel_path in ready_rel}
            for rel_path in ready_rel:
                del self.protected[rel_path]
        
        else:
            ready = set()
        
        if pending > 0:
            logger.info(f"{pending} 个文件等待确认")
        
        return ready