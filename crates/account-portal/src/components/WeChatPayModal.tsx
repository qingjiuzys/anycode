import { useEffect, useRef, useState } from "react";
import { api, PaymentOrder } from "../api";
import { formatMessage, useT } from "../i18n/context";

type Props = {
  order: PaymentOrder;
  onClose: () => void;
  onPaid: () => void;
};

export function WeChatPayModal({ order, onClose, onPaid }: Props) {
  const t = useT();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState(order.status);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!order.code_url || !canvasRef.current) return;
    void import("qrcode").then((QRCode) => {
      void QRCode.toCanvas(canvasRef.current, order.code_url!, {
        width: 220,
        margin: 1,
      });
    });
  }, [order.code_url]);

  useEffect(() => {
    if (status === "paid") return;
    const timer = window.setInterval(() => {
      void api
        .getPaymentOrder(order.id)
        .then((res) => {
          setStatus(res.order.status);
          if (res.order.status === "paid") {
            onPaid();
          }
        })
        .catch((e: unknown) => {
          setError(e instanceof Error ? e.message : String(e));
        });
    }, 2500);
    return () => window.clearInterval(timer);
  }, [order.id, onPaid, status]);

  const amountYuan = (order.amount_fen / 100).toFixed(2);

  return (
    <div className="pay-modal-backdrop" role="presentation" onClick={onClose}>
      <div
        className="card pay-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="wechat-pay-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 id="wechat-pay-title">{t("plans.wechatPayTitle")}</h3>
        <p className="muted">
          {formatMessage(t("plans.wechatPayHint"), {
            plan: order.plan,
            amount: amountYuan,
          })}
        </p>
        {order.code_url && status === "pending" && (
          <div className="pay-qr-wrap">
            <canvas ref={canvasRef} aria-label={t("plans.wechatQrAlt")} />
          </div>
        )}
        {status === "paid" && <p className="form-note pay-success">{t("plans.wechatPaid")}</p>}
        {status === "pending" && <p className="muted pay-wait">{t("plans.wechatWaiting")}</p>}
        {error && <p className="form-note">{error}</p>}
        <div className="pay-modal-actions">
          <button className="btn btn-secondary" type="button" onClick={onClose}>
            {t("plans.wechatClose")}
          </button>
        </div>
      </div>
    </div>
  );
}
