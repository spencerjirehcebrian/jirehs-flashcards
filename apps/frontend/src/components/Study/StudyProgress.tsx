interface StudyProgressProps {
  current: number;
  total: number;
}

export function StudyProgress({ current, total }: StudyProgressProps) {
  const percentage = total > 0 ? (current / total) * 100 : 0;

  return (
    <div className="study-progress">
      <div className="progress-bar">
        <div className="progress-fill" style={{ width: `${percentage}%` }} />
      </div>
      <div className="progress-text">
        {current} / {total}
      </div>
    </div>
  );
}
