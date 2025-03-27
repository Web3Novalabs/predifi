function PoolTypes() {
    return (
        <section className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
            <div className="space-y-8">
                {/* Win Bet Pool Type */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-20 pb-10 border-b border-[#131842]">
                    <div className="grid gap-4">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Win Bet (Main Pool Type)
                        </h1>
                        <p className="text-base leading-relaxed">
                            The Win Bet is a straightforward prediction pool
                            where participants choose between two clear
                            outcomes. This format is ideal for events with
                            definitive winners, such as sports matches,
                            political elections, or game show outcomes. Win Bet
                            establishes a clear and simple entry point for
                            users, making it the cornerstone of PredFi's
                            prediction markets.
                        </p>
                    </div>
                    <div className="leading-relaxed grid gap-4">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Use case
                        </h1>
                        <div className="space-y-2">
                            <p>
                                - Predict the winner of the FIFA World Cup
                                final: Team A vs. Team B
                            </p>
                            <p>
                                - Predict the outcome of a boxing match: Fighter
                                A vs. Fighter B.
                            </p>
                        </div>
                    </div>
                </div>

                {/* Opinion-Based Prediction Pool Type */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-20 pb-10 border-b border-[#131842]">
                    <div className="grid gap-4 md:order-2">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Opinion-Based Prediction (Secondary Pool Type)
                        </h1>
                        <p className="text-base leading-relaxed">
                            This pool format focuses on opinion-based events
                            where there isn't a definitive answer. Instead,
                            participants place bets on subjective topics. The
                            outcome with the most votes at the end of the event
                            wins. Fosters engagement by involving communities in
                            fun and subjective debates.
                        </p>
                    </div>
                    <div className="leading-relaxed grid gap-4 md:order-1">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Use case
                        </h1>
                        <div className="space-y-2">
                            <p>
                                - &quot;Who is the GOAT of football: Messi or
                                Ronaldo?&quot;
                            </p>
                            <p>
                                - &quot;Which song will top the charts this
                                week: Song A or Song B?&quot;
                            </p>
                        </div>
                    </div>
                </div>

                {/* Over/Under Pools */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-20 pb-10 border-b border-[#131842]">
                    <div className="grid gap-4">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Over/Under Pools
                        </h1>
                        <p className="text-base leading-relaxed">
                            In Over/Under pools, participants bet on whether an
                            event&apos;s outcome will be above or below a
                            specified threshold. An example use case is
                            predicting the total goals in a football match:
                            Over/Under 2.5 goals.
                        </p>
                    </div>
                    <div className="leading-relaxed grid gap-4">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Use case
                        </h1>
                        <div className="space-y-2">
                            <p>
                                - With Over 2.5 goals, you win if the total
                                goals scored is 3 or more (e.g., 2-1 or 3-2).
                            </p>
                            <p>
                                - With Under 2.5 goals, you win if the total
                                goals scored is 2 or fewer (e.g., 0-0 or 1-1).
                            </p>
                        </div>
                    </div>
                </div>

                {/* Parlay Pools */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-20 pb-10">
                    <div className="grid gap-4 md:order-2">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Parlay Pools
                        </h1>
                        <p className="text-base leading-relaxed">
                            Parlay pools combine multiple bets into one.
                            Participants must correctly predict the outcomes of
                            all events in the parlay to win. While the risk is
                            higher, the potential rewards are significantly
                            greater.
                        </p>
                    </div>
                    <div className="leading-relaxed grid gap-4 md:order-1">
                        <h1 className="font-semibold text-xl md:text-2xl">
                            Use case
                        </h1>
                        <div className="space-y-2">
                            <p>
                                - Predict the outcomes of multiple football
                                matches in a single pool: Team A, Team C, and
                                Team E all to win.
                            </p>
                            <p>
                                - Combine predictions from different events:
                                Predict the winner of a basketball match and the
                                top scorer in a tennis match.
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}

export default PoolTypes;
