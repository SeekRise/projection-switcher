import { Switch } from "../components/Switch";

interface SettingsPanelProps {
  autostart: boolean | null;
  onToggleAutostart: (next: boolean) => void;
}

export function SettingsPanel({ autostart, onToggleAutostart }: SettingsPanelProps) {
  return (
    <div className="flex flex-col gap-5">
      <header>
        <h2 className="text-lg font-semibold tracking-tight">设置</h2>
        <p className="mt-0.5 text-xs text-muted">后台运行与开机自启动</p>
      </header>

      <section className="border-t border-border pt-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="text-xs">开机自启动</div>
            <p className="mt-0.5 text-[11px] leading-4 text-muted">
              开机后静默在后台启动，仅托盘图标与快捷键就绪
            </p>
          </div>
          <Switch
            checked={autostart === true}
            disabled={autostart === null}
            onChange={onToggleAutostart}
            label="开机自启动"
          />
        </div>
      </section>

      <section className="border-t border-border pt-4">
        <div className="mb-2 text-xs text-muted">后台运行</div>
        <p className="text-[11px] leading-5 text-muted">
          关闭窗口将最小化到系统托盘继续运行，两个全局快捷键保持生效；
          点击托盘图标重新打开主窗口，右键托盘菜单可切换或退出程序。
        </p>
      </section>

      <section className="mt-auto border-t border-border pt-4">
        <div className="mb-2 text-xs text-muted">数据位置</div>
        <p className="text-[11px] leading-5 text-muted">
          配置：%APPDATA%\ProjectionSwitcher.json
          <br />
          投影学习状态：%APPDATA%\DisplaySwitchToggle.state
        </p>
      </section>
    </div>
  );
}
