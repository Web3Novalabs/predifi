function PoolTypes() {
  return (
    <section className="grid gap-7">
      <div className="grid grid-cols-2 gap-20 mb-4 border-b border-[#131842] pb-10">
        <div className="grid gap-1">
          <h1 className="font-semibold text-xl">Win Bet (Main Pool Type)</h1>
          <p>
            The Win Bet is a straightforward prediction pool where participants
            choose between two clear outcomes. This format is ideal for events
            with definitive winners, such as sports matches, political
            elections, or game show outcomes. Win Bet establishes a clear and
            simple entry point for users, making it the cornerstone of PredFi’s
            prediction markets.
          </p>
        </div>
        <div className="leading-[18.77px] grid gap-1">
          <h1 className="font-semibold text-xl">Use case</h1>
          <p>
            - Predict the winner of the FIFA World Cup final: Team A vs. Team B
          </p>
          <p>
            - Predict the outcome of a boxing match: Fighter A vs. Fighter B.
          </p>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-20 mb-4 pb-10 order-1">
        <div className="grid gap-1">
          <h1 className="font-semibold text-xl">
            Opinion-Based Prediction (Secondary Pool Type)
          </h1>
          <p>
            This pool format focuses on opinion-based events where there isn’t a
            definitive answer. Instead, participants place bets on subjective
            topics. The outcome with the most votes at the end of the event
            wins. Fosters engagement by involving communities in fun and
            subjective debates.
          </p>
        </div>
        <div className="leading-[18.77px] grid gap-1">
          <h1 className="font-semibold text-xl">Use case</h1>
          <p>- &quot;Who is the GOAT of football: Messi or Ronaldo?&quot;</p>
          <p>
            - &quot;Which song will top the charts this week: Song A or Song
            B?&quot;
          </p>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-20 mb-4 border-b border-[#131842] pb-10">
        <div className="grid gap-1">
          <h1 className="font-semibold text-xl">Over/Under Pools </h1>
          <p>
            In Over/Under pools, participants bet on whether an event&apos;s
            outcome will be above or below a specified threshold. An example use
            case is predicting the total goals in a football match: Over/Under
            2.5 goals.
          </p>
        </div>
        <div className="leading-[18.77px] grid gap-1">
          <h1 className="font-semibold text-xl">Use case</h1>
          <p>
            - With Over 2.5 goals, you win if the total goals scored is 3 or
            more (e.g., 2-1 or 3-2).
          </p>
          <p>
            - With Under 2.5 goals, you win if the total goals scored is 2 or
            fewer (e.g., 0-0 or 1-1).
          </p>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-20 mb-4 border-b border-[#131842] pb-10">
        <div className="grid gap-1 order-1">
          <h1 className="font-semibold text-xl">Parlay Pools </h1>
          <p>
            Parlay pools combine multiple bets into one. Participants must
            correctly predict the outcomes of all events in the parlay to win.
            While the risk is higher, the potential rewards are significantly
            greater.
          </p>
        </div>
        <div className="leading-[18.77px] grid gap-1">
          <h1 className="font-semibold text-xl">Use case</h1>
          <p>
            - Predict the outcomes of multiple football matches in a single
            pool: Team A, Team C, and Team E all to win.
          </p>
          <p>
            - Combine predictions from different events: Predict the winner of a
            basketball match and the top scorer in a tennis match.
          </p>
        </div>
      </div>
    </section>
  );
}
export default PoolTypes;
