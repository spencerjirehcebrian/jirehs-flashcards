import type { CompareAnswerResponse, DiffSegment } from '@jirehs-flashcards/shared-types';

interface AnswerComparisonProps {
  result: CompareAnswerResponse;
  correctAnswer: string;
}

export function AnswerComparison({ result, correctAnswer }: AnswerComparisonProps) {
  const matchingModeLabel =
    result.matching_mode === 'exact'
      ? 'Exact'
      : result.matching_mode === 'case_insensitive'
      ? 'Case Insensitive'
      : 'Fuzzy';

  return (
    <div className={`answer-comparison ${result.is_correct ? 'correct' : 'incorrect'}`}>
      <div className="comparison-header">
        <span className={`comparison-result ${result.is_correct ? 'correct' : 'incorrect'}`}>
          {result.is_correct ? 'Correct!' : 'Incorrect'}
        </span>
        {result.matching_mode === 'fuzzy' && (
          <span className="comparison-similarity">
            {Math.round(result.similarity * 100)}% match
          </span>
        )}
      </div>

      <div className="comparison-section">
        <div className="comparison-label">Your Answer</div>
        <div className="comparison-diff">
          {result.diff.map((segment, index) => (
            <DiffSpan key={index} segment={segment} />
          ))}
        </div>
      </div>

      <div className="comparison-section">
        <div className="comparison-label">Correct Answer</div>
        <div className="comparison-text">{correctAnswer}</div>
      </div>

      <div className="comparison-mode">
        Matching: {matchingModeLabel}
      </div>
    </div>
  );
}

function DiffSpan({ segment }: { segment: DiffSegment }) {
  const className =
    segment.diff_type === 'Same'
      ? 'diff-same'
      : segment.diff_type === 'Added'
      ? 'diff-added'
      : 'diff-removed';

  return <span className={className}>{segment.text} </span>;
}
