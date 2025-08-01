#!/usr/bin/env python3
import requests
import time
import subprocess
from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.common.by import By
import tempfile

# 配置参数
CHECK_INTERVAL = 60  # 检测间隔(秒)
CHECK_URL = "http://www.baidu.com"  # 用于检测网络连通性的URL
TIMEOUT = 5  # 网络检测超时时间
MAX_RETRIES = 3  # 最大重试次数
LOG_FILE = "/home/sun/campus_network.log"  # 日志文件路径

# 校园网登录信息 (建议从配置文件或环境变量读取)
CAMPUS_NETWORK = {
    "login_url": "https://w.xidian.edu.cn/srun_portal_pc?ac_id=8&theme=pro",
    "username": "",
    "password": "",
    "username_field": "username",  # 用户名输入框元素ID
    "password_field": "password",  # 密码输入框元素ID
    "submit_button": "login-account"   # 登录按钮元素ID
}

def setup_selenium():
    """配置Selenium WebDriver"""
    user_data_dir = tempfile.mkdtemp()
    chrome_options = Options()
    chrome_options.add_argument("--headless")
    chrome_options.add_argument("--no-sandbox")
    chrome_options.add_argument("--disable-dev-shm-usage")
    chrome_options.add_argument("--disable-gpu")
    chrome_options.add_argument("--chrome-skip-compat-layer-relaunch")
    chrome_options.add_argument(f"--user-data-dir={user_data_dir}")
    chrome_options.add_argument("--window-size=1920,1080")
    
    # 对于Linux服务器，可能需要指定chromedriver路径
    driver = webdriver.Chrome(options=chrome_options
    )
    return driver

def login_campus_network(driver):
    """执行校园网登录"""
    try:
        driver.get(CAMPUS_NETWORK["login_url"])
        time.sleep(2)  # 等待页面加载
        
        # 填写登录表单
        driver.find_element(By.ID, "username").send_keys(CAMPUS_NETWORK["username"])
        driver.find_element(By.ID, "password").send_keys(CAMPUS_NETWORK["password"])
        driver.find_element(By.ID, "login-account").click()
        
        time.sleep(2)  # 等待登录完成
        return True
    except Exception as e:
        log_message(f"登录失败: {str(e)}")
        return False

def check_network(driver):
    """检查网络连通性"""
    try:
        driver.get(CAMPUS_NETWORK["login_url"])
        time.sleep(2)
        logout_btn = driver.find_element(By.ID, "logout")
        return logout_btn is not None
    except Exception as e:
        log_message(f"登录状态检测失败: {str(e)}")
        return False


def log_message(message):
    """记录日志"""
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    with open(LOG_FILE, "a") as f:
        f.write(f"[{timestamp}] {message}\n")
    print(f"[{timestamp}] {message}")

def main():
    log_message("校园网自动登录服务启动")
    
    retry_count = 0
    driver = setup_selenium()
    
    try:
        while True:
            if not check_network(driver):
                log_message("网络连接断开，尝试登录校园网")
                if login_campus_network(driver):
                    log_message("校园网登录成功")
                    retry_count = 0
                else:
                    retry_count += 1
                    log_message(f"登录失败，重试次数: {retry_count}/{MAX_RETRIES}")
                    
                    if retry_count >= MAX_RETRIES:
                        log_message("达到最大重试次数，等待下次检测")
                        retry_count = 0
            else:
                if retry_count > 0:
                    log_message("网络已恢复")
                retry_count = 0
            
            time.sleep(CHECK_INTERVAL)
    except KeyboardInterrupt:
        log_message("服务手动停止")
    except Exception as e:
        log_message(f"服务异常: {str(e)}")
    finally:
        driver.quit()
        log_message("服务退出")

if __name__ == "__main__":
    main()
