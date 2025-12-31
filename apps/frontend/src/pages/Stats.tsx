import { useParams, Link } from 'react-router-dom';
import { useDeckStats, useStudyStats, useCalendarData } from '../hooks/useStats';

export function Stats() {
  const { deckPath } = useParams<{ deckPath?: string }>();
  const decodedPath = deckPath ? decodeURIComponent(deckPath) : undefined;

  const { data: deckStats, isLoading: isDeckLoading } = useDeckStats(decodedPath);
  const { data: studyStats, isLoading: isStudyLoading } = useStudyStats();
  const { data: calendarData, isLoading: isCalendarLoading } = useCalendarData(90);

  const isLoading = isDeckLoading || isStudyLoading || isCalendarLoading;

  if (isLoading) {
    return <div className="loading">Loading statistics...</div>;
  }

  return (
    <div className="stats-page">
      <div className="stats-header">
        <h1>{decodedPath ? `Stats: ${decodedPath}` : 'Statistics'}</h1>
        {decodedPath && (
          <Link to="/stats" className="button-secondary">
            View All Stats
          </Link>
        )}
      </div>

      <div className="stats-grid">
        {/* Study Stats Card */}
        {studyStats && (
          <div className="stats-card">
            <h2>Today</h2>
            <div className="stats-row">
              <div className="stat-item">
                <span className="stat-value">{studyStats.reviews_today}</span>
                <span className="stat-label">Reviews</span>
              </div>
              <div className="stat-item">
                <span className="stat-value">{studyStats.new_today}</span>
                <span className="stat-label">Cards Studied</span>
              </div>
            </div>
          </div>
        )}

        {/* Streak Card */}
        {studyStats && (
          <div className="stats-card">
            <h2>Streak</h2>
            <div className="stats-row">
              <div className="stat-item large">
                <span className="stat-value streak">{studyStats.streak_days}</span>
                <span className="stat-label">Day{studyStats.streak_days !== 1 ? 's' : ''}</span>
              </div>
            </div>
          </div>
        )}

        {/* Deck Stats Card */}
        {deckStats && (
          <div className="stats-card">
            <h2>Cards</h2>
            <div className="stats-row">
              <div className="stat-item">
                <span className="stat-value">{deckStats.total_cards}</span>
                <span className="stat-label">Total</span>
              </div>
              <div className="stat-item">
                <span className="stat-value new">{deckStats.new_cards}</span>
                <span className="stat-label">New</span>
              </div>
              <div className="stat-item">
                <span className="stat-value learning">{deckStats.learning_cards}</span>
                <span className="stat-label">Learning</span>
              </div>
              <div className="stat-item">
                <span className="stat-value review">{deckStats.review_cards}</span>
                <span className="stat-label">Review</span>
              </div>
            </div>
          </div>
        )}

        {/* Performance Card */}
        {studyStats && (
          <div className="stats-card">
            <h2>Performance</h2>
            <div className="stats-row">
              <div className="stat-item">
                <span className="stat-value">
                  {Math.round(studyStats.retention_rate * 100)}%
                </span>
                <span className="stat-label">Retention</span>
              </div>
              <div className="stat-item">
                <span className="stat-value">{studyStats.total_reviews}</span>
                <span className="stat-label">Total Reviews</span>
              </div>
            </div>
          </div>
        )}

        {/* Deck Performance Card */}
        {deckStats && (
          <div className="stats-card">
            <h2>Card Performance</h2>
            <div className="stats-row">
              <div className="stat-item">
                <span className="stat-value">{deckStats.average_ease.toFixed(2)}</span>
                <span className="stat-label">Avg Ease</span>
              </div>
              <div className="stat-item">
                <span className="stat-value">{deckStats.average_interval.toFixed(1)}</span>
                <span className="stat-label">Avg Interval (days)</span>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Calendar Heatmap */}
      {calendarData && calendarData.length > 0 && (
        <div className="stats-card calendar-card">
          <h2>Activity (Last 90 Days)</h2>
          <div className="calendar-heatmap">
            {calendarData.map((day) => (
              <div
                key={day.date}
                className={`calendar-day level-${getLevel(day.reviews)}`}
                title={`${day.date}: ${day.reviews} reviews`}
              />
            ))}
          </div>
          <div className="calendar-legend">
            <span>Less</span>
            <div className="calendar-day level-0" />
            <div className="calendar-day level-1" />
            <div className="calendar-day level-2" />
            <div className="calendar-day level-3" />
            <div className="calendar-day level-4" />
            <span>More</span>
          </div>
        </div>
      )}
    </div>
  );
}

function getLevel(reviews: number): number {
  if (reviews === 0) return 0;
  if (reviews < 10) return 1;
  if (reviews < 30) return 2;
  if (reviews < 60) return 3;
  return 4;
}
