package net.snowflake.client.arch;

import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.classes;
import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.methods;
import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.noClasses;

import com.tngtech.archunit.base.DescribedPredicate;
import com.tngtech.archunit.core.domain.JavaClasses;
import com.tngtech.archunit.core.domain.JavaConstructorCall;
import com.tngtech.archunit.core.domain.JavaMethod;
import com.tngtech.archunit.core.importer.ClassFileImporter;
import com.tngtech.archunit.core.importer.ImportOption;
import com.tngtech.archunit.lang.ArchCondition;
import com.tngtech.archunit.lang.ConditionEvents;
import com.tngtech.archunit.lang.SimpleConditionEvent;
import java.sql.SQLException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.decorator.AbstractDecorator;
import net.snowflake.client.internal.api.implementation.exception.DriverRuntimeException;
import org.junit.jupiter.api.Test;

/**
 * Guards the runtime exception-carrier model. {@code DriverRuntimeException.toSQLException()} is
 * already compiler-enforced (a new carrier without a mapping will not compile); these rules cover
 * the structural invariants the compiler cannot. The decorator boundary invariant is behavioural,
 * not structural, and is covered by a separate reflection test.
 */
class ExceptionModelArchTest {

  private static final JavaClasses PRODUCTION_CLASSES =
      new ClassFileImporter()
          .withImportOption(ImportOption.Predefined.DO_NOT_INCLUDE_TESTS)
          .importPackages("net.snowflake.client");

  private static final String[] IMPL_TIER_PACKAGES = {
    "net.snowflake.client.internal.api.implementation..",
    "net.snowflake.client.internal.core.arrow..",
  };
  private static final String EXCEPTION_PKG =
      "net.snowflake.client.internal.api.implementation.exception..";
  // Pooling wraps *already-decorated* physical connections, so it legitimately handles the checked
  // SQLException and re-surfaces it via SFSQLException.surfacing — see LogicalConnection.
  private static final String POOLING_PKG =
      "net.snowflake.client.internal.api.implementation.pooling..";
  private static final String PUBLIC_API_PKG = "net.snowflake.client.api..";

  @Test
  void shouldNotDeclareThrowsSqlExceptionInImplTier() {
    ArchCondition<JavaMethod> declareThrowsSqlException =
        new ArchCondition<JavaMethod>("declare throws java.sql.SQLException") {
          @Override
          public void check(JavaMethod method, ConditionEvents events) {
            boolean leaks =
                method.getThrowsClause().getTypes().stream()
                    .anyMatch(type -> type.isAssignableTo(SQLException.class));
            if (leaks) {
              events.add(
                  SimpleConditionEvent.violated(
                      method, method.getFullName() + " declares throws SQLException"));
            }
          }
        };

    methods()
        .that()
        .areDeclaredInClassesThat()
        .resideInAnyPackage(IMPL_TIER_PACKAGES)
        // The generated Decorated* wrappers ARE the boundary: they implement the java.sql
        // interfaces and re-expose the checked SQLException on purpose. Only the raw impls behind
        // them must be free of it.
        .and()
        .areDeclaredInClassesThat()
        .areNotAssignableTo(AbstractDecorator.class)
        .and()
        .areDeclaredInClassesThat()
        .resideOutsideOfPackage(EXCEPTION_PKG)
        .and()
        .areDeclaredInClassesThat()
        .resideOutsideOfPackage(POOLING_PKG)
        // SnowflakeResultSetSerializableImpl is Serializable, so it cannot be an AbstractDecorator;
        // it is documented as its own exception-translation boundary routing through
        // SqlExceptionMapper.call.
        .and()
        .areDeclaredInClassesThat()
        .haveSimpleNameNotEndingWith("SnowflakeResultSetSerializableImpl")
        .should(declareThrowsSqlException)
        .as("impl-tier methods should not declare throws SQLException")
        .because(
            "impl code throws DriverRuntimeException carriers; SqlExceptionMapper rebuilds the"
                + " checked SQLException at the decorator boundary")
        .check(PRODUCTION_CLASSES);
  }

  @Test
  void shouldConstructSnowflakeSqlExceptionOnlyAtTheBoundary() {
    DescribedPredicate<JavaConstructorCall> constructSnowflakeSqlException =
        new DescribedPredicate<JavaConstructorCall>(
            "construct " + SnowflakeSQLException.class.getSimpleName()) {
          @Override
          public boolean test(JavaConstructorCall call) {
            return call.getTarget().getOwner().isAssignableTo(SnowflakeSQLException.class);
          }
        };

    noClasses()
        .that()
        // SnowflakeSQLException's own static factories live here.
        .resideOutsideOfPackage("net.snowflake.client.api.exception..")
        .and()
        .resideOutsideOfPackage(EXCEPTION_PKG)
        .and()
        .resideOutsideOfPackage(POOLING_PKG)
        // Driver.connect() is the top-level entry point and is not itself decorated; it translates
        // via SqlExceptionMapper.call and throws directly. TODO: route through a carrier too.
        .and()
        .haveSimpleNameNotEndingWith("SnowflakeDriver")
        .should()
        .callConstructorWhere(constructSnowflakeSqlException)
        .as("only the exception boundary should construct SnowflakeSQLException")
        .because(
            "byte-exact / legacy-parity construction must funnel through the exception package")
        .check(PRODUCTION_CLASSES);
  }

  @Test
  void shouldMakeEveryExceptionCarrierUnchecked() {
    classes()
        .that()
        .resideInAPackage(EXCEPTION_PKG)
        .and()
        .areAssignableTo(Throwable.class)
        .should()
        .beAssignableTo(DriverRuntimeException.class)
        .as("throwables in the exception package should extend DriverRuntimeException")
        .because(
            "a checked carrier would force throws SQLException back into the impl tier it was"
                + " removed from")
        .check(PRODUCTION_CLASSES);
  }

  @Test
  void shouldNotLeakCarrierTypesThroughThePublicApi() {
    noClasses()
        .that()
        .resideInAPackage(PUBLIC_API_PKG)
        .should()
        .dependOnClassesThat()
        .areAssignableTo(DriverRuntimeException.class)
        .as("public api classes should not depend on internal carrier types")
        .because(
            "carriers are translated to SnowflakeSQLException before crossing the api boundary")
        .check(PRODUCTION_CLASSES);
  }
}
