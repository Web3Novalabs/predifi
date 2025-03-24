const TabNavigation = () => (
    <div className="flex gap-[13px]">
      {["Dashboard", "Profile"].map((tab) => (
        <button
          key={tab}
          className="w-[170px] hidden md:block h-[44px] bg-[#373737] hover:text-[whitesmoke] transition-all duration-200 font-['Work_sans'] font-[300] rounded-t-[48px] text-[#CCCCCC]"
        >
          {tab}
        </button>
      ))}
      <div className=" w-[150px] max-h-[44px] px-2 py-1 border flex items-center  justify-start md:hidden rounded-full  border-[#8f8f8f]">
        <select className="w-full bg-transparent text-[#CCCCCC] font-['Work_sans'] outline-none font-[300]  ">
          {["Dashboard", "Profile"].map((tab) => (
            <option key={tab} value={tab}>
              {tab}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
  
  export default TabNavigation;
  