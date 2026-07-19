import { useEffect, useState } from "react";
import {
  approveIdentity,
  fetchIdentityReviews,
  rejectIdentity,
  revealIdentity,
  type IdentityReview,
} from "../api";

export default function IdentityReviewsPage() {
  const [reviews, setReviews] = useState<IdentityReview[]>([]);
  const [error, setError] = useState("");
  const [revealed, setRevealed] = useState<Record<string, { legal_name: string; id_number: string }>>({});

  const reload = () => {
    void fetchIdentityReviews()
      .then((result) => setReviews(result.reviews))
      .catch((reason) => setError(String(reason)));
  };

  useEffect(reload, []);

  const reject = async (review: IdentityReview) => {
    const reason = window.prompt("请输入驳回原因（将向用户展示）");
    if (!reason?.trim()) return;
    try {
      await rejectIdentity(review.id, reason.trim());
      reload();
    } catch (reason) {
      setError(String(reason));
    }
  };

  const reveal = async (review: IdentityReview) => {
    const purpose = window.prompt("请输入查看实名明文的审核目的（操作将被审计）");
    if (!purpose?.trim()) return;
    try {
      const value = await revealIdentity(review.id, purpose.trim());
      setRevealed((current) => ({ ...current, [review.id]: value }));
    } catch (reason) {
      setError(String(reason));
    }
  };

  return (
    <section>
      <h1>实名认证审核</h1>
      <p className="ops-muted">
        默认仅显示脱敏身份证号。首版不接收或存储证件图片，审核决定会记录操作日志。
      </p>
      {error && <p className="ops-error" role="alert">{error}</p>}
      <div className="ops-card ops-table-wrap">
        <table>
          <thead>
            <tr>
              <th>账号</th>
              <th>身份证</th>
              <th>状态</th>
              <th>提交时间</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {reviews.map((review) => (
              <tr key={review.id}>
                <td>{review.email}</td>
                <td>
                  {revealed[review.id]
                    ? `${revealed[review.id].legal_name} · ${revealed[review.id].id_number}`
                    : review.id_number_masked}
                </td>
                <td>{review.status}</td>
                <td>{new Date(review.submitted_at).toLocaleString()}</td>
                <td>
                  {review.status === "pending" && (
                    <div className="ops-actions">
                      <button type="button" onClick={() => void reveal(review)}>
                        审核查看
                      </button>
                      <button type="button" onClick={() => void approveIdentity(review.id).then(reload)}>
                        通过
                      </button>
                      <button type="button" onClick={() => void reject(review)}>
                        驳回
                      </button>
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
